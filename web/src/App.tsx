import { useState, type ReactNode } from "react";

import { strings } from "./i18n/strings";
import "./app.css";

type IconName =
  | "archive"
  | "back"
  | "chevron"
  | "clock"
  | "dots"
  | "forward"
  | "print"
  | "reply"
  | "star"
  | "trash";

type Message = {
  id: number;
  sender: string;
  initials: string;
  email: string;
  time: string;
  preview: string;
  body: string;
};

const messages: Message[] = [
  {
    id: 1,
    sender: strings.sampleSender,
    initials: strings.sampleInitials,
    email: strings.sampleEmail,
    time: strings.sampleTimeFirst,
    preview: strings.samplePreviewFirst,
    body: strings.sampleBodyFirst,
  },
  {
    id: 2,
    sender: strings.sampleSender,
    initials: strings.sampleInitials,
    email: strings.sampleEmail,
    time: strings.sampleTimeLatest,
    preview: strings.samplePreviewSecond,
    body: strings.sampleBodySecond,
  },
  {
    id: 3,
    sender: strings.sampleSender,
    initials: strings.sampleInitials,
    email: strings.sampleEmail,
    time: strings.sampleTimeLatest,
    preview: strings.samplePreviewLatest,
    body: strings.sampleBodyLatest,
  },
];

function Icon({ name }: { name: IconName }) {
  const paths: Record<IconName, ReactNode> = {
    archive: (
      <>
        <path d="M4 7h16" />
        <path d="M5 7v12h14V7" />
        <path d="M3 3h18v4H3z" />
        <path d="M9 11h6" />
      </>
    ),
    back: (
      <>
        <path d="m15 18-6-6 6-6" />
        <path d="M9 12h11" />
      </>
    ),
    chevron: <path d="m9 18 6-6-6-6" />,
    clock: (
      <>
        <circle cx="12" cy="12" r="9" />
        <path d="M12 7v5l3 2" />
      </>
    ),
    dots: (
      <>
        <circle cx="5" cy="12" r="1" fill="currentColor" stroke="none" />
        <circle cx="12" cy="12" r="1" fill="currentColor" stroke="none" />
        <circle cx="19" cy="12" r="1" fill="currentColor" stroke="none" />
      </>
    ),
    forward: (
      <>
        <path d="m14 8 5 4-5 4" />
        <path d="M19 12H9a5 5 0 0 0-5 5" />
      </>
    ),
    print: (
      <>
        <path d="M7 9V3h10v6" />
        <path d="M7 18H5a2 2 0 0 1-2-2v-5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2v5a2 2 0 0 1-2 2h-2" />
        <path d="M7 14h10v7H7z" />
      </>
    ),
    reply: (
      <>
        <path d="m10 8-5 4 5 4" />
        <path d="M5 12h10a5 5 0 0 1 5 5" />
      </>
    ),
    star: <path d="m12 3 2.8 5.7 6.2.9-4.5 4.4 1.1 6.2-5.6-3-5.6 3 1.1-6.2L3 9.6l6.2-.9z" />,
    trash: (
      <>
        <path d="M4 7h16" />
        <path d="M9 3h6l1 4H8z" />
        <path d="m6 7 1 14h10l1-14" />
        <path d="M10 11v6M14 11v6" />
      </>
    ),
  };

  return (
    <svg aria-hidden="true" viewBox="0 0 24 24">
      {paths[name]}
    </svg>
  );
}

function IconButton({
  label,
  icon,
  emphasis = false,
}: {
  label: string;
  icon: IconName;
  emphasis?: boolean;
}) {
  return (
    <button
      className={emphasis ? "icon-button icon-button--emphasis" : "icon-button"}
      type="button"
      aria-label={label}
      title={label}
    >
      <Icon name={icon} />
    </button>
  );
}

function MessageCard({
  message,
  expanded,
  onToggle,
}: {
  message: Message;
  expanded: boolean;
  onToggle: () => void;
}) {
  const contentId = `message-content-${message.id}`;

  return (
    <article className={expanded ? "message-card message-card--open" : "message-card"}>
      <button
        className="message-summary"
        type="button"
        aria-expanded={expanded}
        aria-controls={contentId}
        onClick={onToggle}
      >
        <span className="avatar" aria-hidden="true">
          {message.initials}
        </span>
        <span className="message-identity">
          <span className="sender-row">
            <strong>{message.sender}</strong>
            {!expanded && <span className="message-preview">{message.preview}</span>}
          </span>
          <span className="recipient">
            {expanded ? `${message.email} · ${strings.toMe}` : strings.toMe}
          </span>
        </span>
        <span className="message-meta">
          <time>{message.time}</time>
          <span className="summary-chevron">
            <Icon name="chevron" />
          </span>
        </span>
      </button>

      {expanded && (
        <div className="message-content" id={contentId}>
          <div className="message-actions">
            <IconButton label={strings.star} icon="star" />
            <IconButton label={strings.reply} icon="reply" />
            <IconButton label={strings.moreActions} icon="dots" />
          </div>
          <p>{message.body}</p>
          <div className="reply-actions">
            <button className="action-button action-button--primary" type="button">
              <Icon name="reply" />
              {strings.reply}
            </button>
            <button className="action-button" type="button">
              <Icon name="forward" />
              {strings.forward}
            </button>
          </div>
        </div>
      )}
    </article>
  );
}

export function App() {
  const [openMessages, setOpenMessages] = useState<Set<number>>(
    () => new Set([messages.at(-1)?.id ?? 1]),
  );

  const toggleMessage = (id: number) => {
    setOpenMessages((current) => {
      const next = new Set(current);
      if (next.has(id)) {
        next.delete(id);
      } else {
        next.add(id);
      }
      return next;
    });
  };

  return (
    <main className="thread-page">
      <div className="thread-shell">
        <nav className="top-bar" aria-label={strings.threadNavigation}>
          <button className="back-button" type="button">
            <Icon name="back" />
            <span>{strings.backToInbox}</span>
          </button>
          <div className="top-actions">
            <IconButton label={strings.archive} icon="archive" />
            <IconButton label={strings.snooze} icon="clock" />
            <IconButton label={strings.deleteMessage} icon="trash" />
            <span className="action-divider" aria-hidden="true" />
            <IconButton label={strings.print} icon="print" />
            <IconButton label={strings.moreActions} icon="dots" />
          </div>
        </nav>

        <header className="thread-header">
          <div>
            <div className="eyebrow">
              <span className="inbox-pill">{strings.inbox}</span>
              <span>{strings.messageCount(messages.length)}</span>
            </div>
            <h1>{strings.sampleSubject}</h1>
          </div>
          <button className="view-toggle" type="button">
            {strings.conversationView}
            <Icon name="chevron" />
          </button>
        </header>

        <section className="message-stack" aria-label={strings.messages}>
          {messages.map((message) => (
            <MessageCard
              key={message.id}
              message={message}
              expanded={openMessages.has(message.id)}
              onToggle={() => toggleMessage(message.id)}
            />
          ))}
        </section>
      </div>
    </main>
  );
}
