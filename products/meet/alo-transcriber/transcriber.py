"""Multi-participant STT worker for alo Meet; it never uses an LLM or TTS."""

import asyncio
import logging
import os

from livekit import rtc
from livekit.agents import Agent, AgentServer, AgentSession, AutoSubscribe, JobContext, StopResponse, cli, llm, room_io, utils
from livekit.plugins import deepgram, openai

logger = logging.getLogger("alo-meet-transcriber")


def speech_to_text():
    provider = os.environ.get("ALO_TRANSCRIPTION_PROVIDER", "").strip().lower()
    language = os.environ.get("ALO_TRANSCRIPTION_LANGUAGE", "multi").strip()
    if provider == "deepgram":
        if not os.environ.get("DEEPGRAM_API_KEY"):
            raise RuntimeError("DEEPGRAM_API_KEY is required for the Deepgram transcriber")
        return deepgram.STT(model=os.environ.get("ALO_TRANSCRIPTION_MODEL") or "nova-3", language=language)
    if provider == "openai":
        if not os.environ.get("OPENAI_API_KEY"):
            raise RuntimeError("OPENAI_API_KEY is required for the OpenAI transcriber")
        options = {"model": os.environ.get("ALO_TRANSCRIPTION_MODEL") or "gpt-4o-mini-transcribe"}
        if language != "multi":
            options["language"] = language
        return openai.STT(**options)
    raise RuntimeError("ALO_TRANSCRIPTION_PROVIDER must be 'deepgram' or 'openai'")


class Transcriber(Agent):
    def __init__(self, participant_identity: str):
        super().__init__(instructions="Transcribe speech only.", stt=speech_to_text())
        self.participant_identity = participant_identity

    async def on_user_turn_completed(self, chat_ctx: llm.ChatContext, new_message: llm.ChatMessage):
        logger.info("final transcript received for %s", self.participant_identity)
        raise StopResponse()


class MultiUserTranscriber:
    def __init__(self, ctx: JobContext):
        self.ctx = ctx
        self.sessions: dict[str, AgentSession] = {}
        self.tasks: set[asyncio.Task] = set()

    def start(self):
        self.ctx.room.on("participant_connected", self.participant_connected)
        self.ctx.room.on("participant_disconnected", self.participant_disconnected)

    def participant_connected(self, participant: rtc.RemoteParticipant):
        if participant.identity in self.sessions:
            return
        task = asyncio.create_task(self.start_session(participant))
        self.tasks.add(task)

        def ready(completed: asyncio.Task):
            try:
                self.sessions[participant.identity] = completed.result()
            finally:
                self.tasks.discard(completed)

        task.add_done_callback(ready)

    def participant_disconnected(self, participant: rtc.RemoteParticipant):
        session = self.sessions.pop(participant.identity, None)
        if session is None:
            return
        task = asyncio.create_task(self.close_session(session))
        self.tasks.add(task)
        task.add_done_callback(self.tasks.discard)

    async def start_session(self, participant: rtc.RemoteParticipant) -> AgentSession:
        session = AgentSession()
        await session.start(
            agent=Transcriber(participant.identity),
            room=self.ctx.room,
            room_options=room_io.RoomOptions(audio_input=True, text_output=True, audio_output=False, participant_identity=participant.identity, text_input=False),
        )
        return session

    async def close_session(self, session: AgentSession):
        await session.drain()
        await session.aclose()

    async def close(self):
        await utils.aio.cancel_and_wait(*self.tasks)
        await asyncio.gather(*(self.close_session(session) for session in self.sessions.values()))
        self.ctx.room.off("participant_connected", self.participant_connected)
        self.ctx.room.off("participant_disconnected", self.participant_disconnected)


server = AgentServer()


@server.rtc_session()
async def meeting_transcription(ctx: JobContext):
    transcriber = MultiUserTranscriber(ctx)
    transcriber.start()
    await ctx.connect(auto_subscribe=AutoSubscribe.AUDIO_ONLY)
    for participant in ctx.room.remote_participants.values():
        transcriber.participant_connected(participant)
    ctx.add_shutdown_callback(transcriber.close)


if __name__ == "__main__":
    cli.run_app(server)
