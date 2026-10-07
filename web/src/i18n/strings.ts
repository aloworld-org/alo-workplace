// All user-facing strings live here (CLAUDE.md: externalized from day
// one — hardcoded English in components is a bug in a European
// product). This module becomes the translation catalog's source when
// real i18n tooling lands with the Phase 2 web shell.
export const strings = {
  appName: "Ficina",
  archive: "Archive",
  backToInbox: "Back to inbox",
  conversationView: "Conversation view",
  deleteMessage: "Delete",
  forward: "Forward",
  inbox: "Inbox",
  messages: "Messages in this conversation",
  moreActions: "More actions",
  print: "Print conversation",
  reply: "Reply",
  sampleBodyFirst: "Threading verification.",
  sampleBodyLatest: "Threading verification.",
  sampleBodySecond: "Threading verification.",
  sampleEmail: "disan@name13ss.com",
  sampleInitials: "DI",
  samplePreviewFirst: "Threading verification",
  samplePreviewLatest: "Threading verification",
  samplePreviewSecond: "Threading verification",
  sampleSender: "Disan",
  sampleSubject: "Thread b2c7 — original",
  sampleTimeFirst: "10:05 AM",
  sampleTimeLatest: "10:06 AM",
  snooze: "Snooze",
  star: "Star message",
  threadNavigation: "Conversation actions",
  toMe: "to me",
  messageCount: (count: number) => `${count} messages`,
} as const;
