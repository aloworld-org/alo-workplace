// The alo workspace product surface: the full suite. Adds the suite modules
// (Docs under Drive, plus the coming-soon Agenda/Chat/Meet), the multi-tenant
// control-plane console, and the Docs-powered equation/code inserts for the
// compose editor.
//
// This is the ONE file that imports the suite-only areas (`../control`,
// `../authoring`). alomails ships the `mail` surface instead and deletes this
// file together with those areas — nothing else in the web app references them.
import { Suspense, lazy, type ComponentType, type ReactNode } from "react";
import {
  BarChart3,
  Boxes,
  Briefcase,
  Building2,
  Code2,
  Globe,
  HardDrive,
  Handshake,
  Landmark,
  Megaphone,
  MessagesSquare,
  Receipt,
  Sigma,
  Users,
  Video,
} from "lucide-react";

import { strings } from "../i18n";
import { AgendaAbsenceProvider } from "../hr/AgendaAbsences";
import { ApprovalsWidget } from "../hr/ApprovalsWidget";
import { TimerWidget } from "../projects/TimerWidget";
import type { ComposeInsert, ProductModule, ProductSurface } from "./types";
import { adminConsole, defaultPath, sharedModules } from "./shared";

// The technical-authoring editor pulls in KaTeX + Prism, so it is code-split:
// those libraries load only when a user inserts an equation/code block, never on
// mail. (The full Docs surface now lives inside Drive as a file type.)
const AuthoringInsertModal = lazy(() =>
  import("../authoring").then((m) => ({ default: m.AuthoringInsertModal })),
);

const DriveModule = lazy(() => import("../drive").then((m) => ({ default: m.DriveModule })));
const BillingModule = lazy(() => import("../billing").then((m) => ({ default: m.BillingModule })));
const ChatModule = lazy(() => import("../chat").then((m) => ({ default: m.ChatModule })));
const MeetModule = lazy(() => import("../meet").then((m) => ({ default: m.MeetModule })));
const CrmModule = lazy(() => import("../crm").then((m) => ({ default: m.CrmModule })));
const FinanceModule = lazy(() => import("../finance").then((m) => ({ default: m.FinanceModule })));
const HrModule = lazy(() => import("../hr").then((m) => ({ default: m.HrModule })));
const InsightsModule = lazy(() => import("../insights").then((m) => ({ default: m.InsightsModule })));
const InventoryModule = lazy(() => import("../inventory").then((m) => ({ default: m.InventoryModule })));
const ProjectsModule = lazy(() => import("../projects").then((m) => ({ default: m.ProjectsModule })));
const SitesModule = lazy(() => import("../sites").then((m) => ({ default: m.SitesModule })));
const CampaignsModule = lazy(() =>
  import("../campaigns").then((m) => ({ default: m.CampaignsModule })),
);
const ControlConsole = lazy(() => import("../control").then((m) => ({ default: m.ControlConsole })));

function ModuleLoading() {
  return <div className="flex h-full min-h-0 flex-1" aria-label={strings.chatLoading} role="status"><aside className="w-96 shrink-0 border-r border-subtle bg-surface p-6"><span className="mb-8 block h-8 w-24 animate-pulse rounded-md bg-raised" /><span className="mb-4 block h-11 animate-pulse rounded-lg bg-raised" /><span className="mb-3 block h-12 animate-pulse rounded-lg bg-raised" /><span className="block h-12 animate-pulse rounded-lg bg-raised" /></aside><main className="flex flex-1 items-center justify-center bg-surface"><span className="text-sm text-tertiary">{strings.chatLoading}</span></main></div>;
}

const deferred = (Component: ComponentType) => () => (
  <Suspense fallback={<ModuleLoading />}>
    <Component />
  </Suspense>
);

/** A compose-insert Modal that reuses the shared authoring modal for `kind`. */
function insertModal(kind: "equation" | "code"): ComposeInsert["Modal"] {
  return function Insert(props: {
    onInsert: (html: string) => void;
    onClose: () => void;
  }) {
    return (
      <Suspense fallback={null}>
        <AuthoringInsertModal
          kind={kind}
          onInsert={props.onInsert}
          onClose={props.onClose}
        />
      </Suspense>
    );
  };
}

const suiteModules: ProductModule[] = [
  // The alomails products (Home/Mail/Agenda/Tasks) come from sharedModules; the
  // workspace adds Drive (alodrives, with its file-hosted documents) and the
  // not-yet-built Chat and Meet. This is why Drive shows on aloworkplace.com but
  // not on the standalone alomails app.
  //
  // The workspace's Agenda additionally carries the HR absence layer (B7.03):
  // the shared module declares the seam, and only the product that has an HR
  // provides the feed — the standalone mail app renders the same calendar with
  // the layer empty.
  ...sharedModules.map((module): ProductModule => {
    if (module.id !== "agenda" || module.element === undefined) return module;
    const Agenda = module.element;
    return {
      ...module,
      element: (): ReactNode => (
        <AgendaAbsenceProvider>
          <Agenda />
        </AgendaAbsenceProvider>
      ),
    };
  }),
  {
    id: "drive",
    path: "/drive",
    label: strings.moduleDrive,
    Icon: HardDrive,
    enabled: true,
    element: deferred(DriveModule),
  },
  // Billing is a workspace module only (ADR 0035): the business suite is what
  // aloworkplace.com sells, and it has no place in the standalone mail app.
  {
    id: "billing",
    path: "/billing",
    label: strings.moduleBilling,
    Icon: Receipt,
    enabled: true,
    element: deferred(BillingModule),
  },
  // CRM is a workspace module only (ADR 0035), beside Billing and for the same
  // reason: the business suite is what aloworkplace.com sells. It sits next to
  // Billing because a won deal becomes a quote there.
  {
    id: "crm",
    path: "/crm",
    label: strings.moduleCrm,
    Icon: Handshake,
    enabled: true,
    element: deferred(CrmModule),
  },
  // Projects is a workspace module only (ADR 0035, wave B3), beside Billing
  // and CRM: client work, the hours worked on it, and the invoice those hours
  // become. It sits after CRM because a won deal becomes an engagement, and
  // before Insights because Insights reads what the three of them record.
  //
  // Its boards are the SAME rows Tasks shows — a client project is a task
  // project with client facts beside it (docs/design/projects.md). Two tabs
  // saying "project" is the honest cost of one project list.
  {
    id: "projects",
    path: "/projects",
    label: strings.moduleProjects,
    Icon: Briefcase,
    enabled: true,
    element: deferred(ProjectsModule),
  },
  // Finance is a workspace module only (ADR 0035, wave B4): the books behind
  // the documents Billing raises. It sits after Projects and before Insights,
  // in the order money moves — a claim, an invoice, the hours behind it, then
  // the ledger they all post to.
  //
  // Its expenses tab is the screen most employees will ever open here; the
  // approver's queues are a tab of the same module rather than a console,
  // because deciding a claim is bookkeeping and not administration
  // (`docs/design/finance.md` § The accountant role).
  {
    id: "finance",
    path: "/finance",
    label: strings.moduleFinance,
    Icon: Landmark,
    enabled: true,
    element: deferred(FinanceModule),
  },
  // Inventory is a workspace module only (ADR 0035, wave B5): what a business
  // buys, keeps and ships. It sits after Finance and before Insights because it
  // is the last of the modules that *record* — a receipt and a delivery are
  // events, like an invoice and a claim — and Insights reads all of them.
  //
  // Its catalog is Billing's product rows seen as things rather than as prices
  // (`docs/design/inventory.md` § The catalog). Two tabs saying "product" is the
  // honest cost of one product record, the same trade Projects makes with Tasks.
  {
    id: "inventory",
    path: "/inventory",
    label: strings.moduleInventory,
    Icon: Boxes,
    enabled: true,
    element: deferred(InventoryModule),
  },
  // HR is a workspace module only (ADR 0035, wave B6): the people a business
  // employs, and the people applying to join it. Unlike the business modules before
  // it, its rail entry is meant for **every member** — the most-used HR screen
  // is an employee's own (`docs/design/hr.md` § Web surface) — so what varies
  // by door is the tabs inside, not whether the module is there at all.
  {
    id: "hr",
    path: "/hr",
    label: strings.moduleHr,
    Icon: Users,
    enabled: true,
    element: deferred(HrModule),
  },
  // Insights is a workspace module only (ADR 0037), and it sits after the
  // modules whose records it reads: a chart here is billing's and CRM's own
  // figures, computed by the server and only drawn in the browser.
  {
    id: "insights",
    path: "/insights",
    label: strings.moduleInsights,
    Icon: BarChart3,
    enabled: true,
    element: deferred(InsightsModule),
  },
  // Sites is a workspace module only (ADR 0036): the public website a business
  // publishes belongs to the suite, not to the standalone mail app.
  {
    id: "sites",
    path: "/sites",
    label: strings.moduleSites,
    Icon: Globe,
    enabled: true,
    element: deferred(SitesModule),
  },
  // Campaigns is a workspace module only (ADR 0044): bulk email to a business's
  // own customers belongs to the suite. The screen reads the audience and the
  // question asked of it — nothing on it sends, because the sending identity
  // needs a second IP that has to be bought, and the wave that builds it says so
  // rather than shipping a button that would poison the invoices.
  //
  // Not in `module_access.rs`'s gated prefixes and deliberately so: an admin
  // switch would need an `AppModule` variant and a CHECK migration on a table
  // this track does not own. The routes are still tenant-scoped and
  // authenticated; what is missing is only the per-user on/off toggle.
  {
    id: "campaigns",
    path: "/campaigns",
    label: strings.moduleCampaigns,
    Icon: Megaphone,
    enabled: true,
    element: deferred(CampaignsModule),
  },
  // Chat is live as of ADR 0038 — built on alo's own store, not Matrix.
  {
    id: "chat",
    path: "/chat",
    label: strings.moduleChat,
    Icon: MessagesSquare,
    enabled: true,
    element: deferred(ChatModule),
  },
  {
    id: "meet",
    path: "/meet",
    label: strings.moduleMeet,
    Icon: Video,
    enabled: true,
    element: deferred(MeetModule),
  },
];

export const surface: ProductSurface = {
  modules: suiteModules,
  consoles: [
    adminConsole,
    {
      path: "/control/*",
      element: deferred(ControlConsole),
      menu: {
        to: "/control",
        label: strings.controlOpen,
        Icon: Building2,
        requires: "operator",
      },
    },
  ],
  composeInserts: [
    {
      id: "equation",
      label: strings.composeInsertEquation,
      Icon: Sigma,
      Modal: insertModal("equation"),
    },
    {
      id: "code",
      label: strings.composeInsertCode,
      Icon: Code2,
      Modal: insertModal("code"),
    },
  ],
  // The running timer, visible from every module: a clock you cannot see from
  // your inbox is a clock you forget to stop. It draws nothing when none runs,
  // which is the ordinary state of a workspace.
  //
  // Beside it, the approvals count (B6.07) — leave, expense claims and
  // timesheet weeks summed across the three modules that hold them, for the
  // people who have to decide them. It draws nothing when nothing is waiting,
  // and nothing at all for the majority who have no queue to work.
  railWidgets: [
    { id: "timer", Widget: TimerWidget },
    { id: "approvals", Widget: ApprovalsWidget },
  ],
  defaultPath,
  brand: {
    headline: () => strings.brandHeadline,
    subtitle: () => strings.brandSubtitle,
    euBadge: () => strings.brandEuBadge,
  },
  // Business/suite product: SSO for company IdPs, bring-your-domain placeholder.
  login: {
    sso: true,
    emailPlaceholder: () => strings.emailPlaceholder,
  },
};
