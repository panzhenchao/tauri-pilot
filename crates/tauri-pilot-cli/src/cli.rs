use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "tauri-pilot", about = "Interactive testing CLI for Tauri apps")]
pub(crate) struct Cli {
    /// Socket path (auto-detected if omitted).
    #[arg(long, env = "TAURI_PILOT_SOCKET")]
    pub socket: Option<PathBuf>,

    /// Output JSON instead of text.
    #[arg(long, global = true)]
    pub json: bool,

    /// Target a specific window by label
    #[arg(long, env = "TAURI_PILOT_WINDOW", global = true)]
    pub window: Option<String>,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub(crate) enum Command {
    /// Start a Model Context Protocol server over stdio.
    Mcp,
    /// List all open windows
    Windows,
    /// Check connectivity with a running Tauri app.
    Ping,
    /// Get app state (url, title, ready).
    State,
    /// Capture an accessibility snapshot of the UI.
    Snapshot {
        #[arg(short, long)]
        interactive: bool,
        #[arg(short, long)]
        selector: Option<String>,
        #[arg(short, long)]
        depth: Option<u8>,
        #[arg(long, value_name = "FILE")]
        save: Option<std::path::PathBuf>,
    },
    /// Compare current page with previous snapshot, showing only differences
    Diff {
        /// Path to a saved snapshot file to compare against
        #[arg(long, value_name = "FILE")]
        r#ref: Option<std::path::PathBuf>,
        /// Only include interactive elements
        #[arg(short, long)]
        interactive: bool,
        /// CSS selector to scope the snapshot
        #[arg(short, long)]
        selector: Option<String>,
        /// Maximum depth to traverse
        #[arg(short, long)]
        depth: Option<u8>,
    },
    /// Click an element.
    Click { target: String },
    /// Clear and fill an input with a value.
    Fill { target: String, value: String },
    /// Type text character by character.
    Type { target: String, text: String },
    /// Press a keyboard key.
    Press { key: String },
    /// Select an option in a <select>.
    Select { target: String, value: String },
    /// Toggle a checkbox.
    Check { target: String },
    /// Scroll the page or an element.
    Scroll {
        direction: String,
        amount: Option<i32>,
        #[arg(long)]
        r#ref: Option<String>,
    },
    /// Drag an element to another element or by offset.
    Drag {
        source: String,
        #[arg(conflicts_with = "offset")]
        target: Option<String>,
        /// Pixel offset as X,Y (e.g., "0,100").
        #[arg(long, value_name = "X,Y", conflicts_with = "target")]
        offset: Option<String>,
    },
    /// Simulate a file drop on an element.
    Drop {
        target: String,
        /// File(s) to drop. Can be repeated.
        #[arg(long, required = true)]
        file: Vec<std::path::PathBuf>,
    },
    /// Get text content of an element.
    Text { target: String },
    /// Get inner HTML (of an element, or full page).
    Html { target: Option<String> },
    /// Get the value of an input/select/textarea.
    Value { target: String },
    /// Get all attributes of an element.
    Attrs { target: String },
    /// Evaluate arbitrary JavaScript.
    /// Pass `-` as the script or omit it to read from stdin.
    Eval {
        /// JavaScript to evaluate. Use `-` or omit to read from stdin.
        script: Option<String>,
    },
    /// Invoke a Tauri IPC command.
    Ipc {
        command: String,
        #[arg(long)]
        args: Option<String>,
    },
    /// Capture a screenshot (PNG).
    Screenshot {
        path: Option<PathBuf>,
        #[arg(long)]
        selector: Option<String>,
    },
    /// Navigate to a URL.
    Navigate { url: String },
    /// Get current URL.
    Url,
    /// Get page title.
    Title,
    /// Wait for an element or condition.
    Wait {
        target: Option<String>,
        #[arg(long)]
        selector: Option<String>,
        #[arg(long)]
        gone: bool,
        #[arg(long, default_value = "10000")]
        timeout: u64,
    },
    /// Watch for DOM mutations and report changes.
    Watch {
        /// CSS selector to scope observation to a subtree.
        #[arg(long)]
        selector: Option<String>,
        /// Maximum wait time in ms. With `--require-mutation`, rejects on timeout
        /// if no mutation occurred; otherwise resolves after `--stable` ms of quiet.
        #[arg(long, default_value = "10000")]
        timeout: u64,
        /// Wait until DOM is stable for N ms (no new mutations).
        #[arg(long, default_value = "300")]
        stable: u64,
        /// Defer the stability timer until at least one mutation occurs.
        /// Use after IPC calls that trigger async re-renders (e.g., React state updates).
        #[arg(long)]
        require_mutation: bool,
    },
    /// Display or stream captured console logs.
    Logs {
        /// Filter by log level (log, info, warn, error).
        #[arg(long, value_parser = ["log", "info", "warn", "error"])]
        level: Option<String>,

        /// Show last N log entries.
        #[arg(long)]
        last: Option<usize>,

        /// Clear the log buffer.
        #[arg(long, conflicts_with = "follow")]
        clear: bool,

        /// Continuously poll for new logs.
        #[arg(long, short = 'f')]
        follow: bool,
    },
    /// Display or stream captured network requests.
    Network {
        /// Filter by URL pattern (substring match).
        #[arg(long)]
        filter: Option<String>,

        /// Show only failed requests (4xx/5xx and network errors).
        #[arg(long)]
        failed: bool,

        /// Show last N requests.
        #[arg(long)]
        last: Option<usize>,

        /// Clear the request buffer.
        #[arg(long, conflicts_with = "follow")]
        clear: bool,

        /// Continuously poll for new requests.
        #[arg(long, short = 'f')]
        follow: bool,
    },
    /// Assert element state
    #[command(subcommand)]
    Assert(AssertKind),
    /// Read and write browser storage (localStorage/sessionStorage).
    Storage(StorageArgs),
    /// Dump all form fields on the page.
    Forms(FormsArgs),
    /// Record interactions for replay
    Record {
        #[command(subcommand)]
        action: RecordAction,
    },
    /// Replay a recorded session
    Replay {
        /// Path to recording file (JSON)
        path: PathBuf,
        /// Export format instead of replaying (e.g., "sh")
        #[arg(long)]
        export: Option<String>,
    },
    /// Execute a declarative scenario from a TOML file.
    Run {
        /// Path to scenario TOML file.
        scenario: PathBuf,
        /// Write `JUnit` XML report to this path.
        #[arg(long, value_name = "FILE")]
        junit: Option<PathBuf>,
        /// Override `fail_fast` setting from the scenario file.
        #[arg(long)]
        no_fail_fast: bool,
    },
    /// AIjia-specific business commands (e2e testing for the AIjia desktop app).
    Aijia {
        #[command(subcommand)]
        command: AijiaCommand,
    },
}

#[derive(Subcommand, Debug)]
pub(crate) enum AijiaCommand {
    /// Click the sidebar "新任务" entry to route to the new-task page.
    NewTask {
        /// Wait until chatStore.messages is empty (up to 2s) before returning.
        /// new-task is a lazy create — without this flag, `where.sessionId`
        /// and `where.messageCount` may still reflect the previous session
        /// until the user calls `send`.
        #[arg(long, default_value_t = false)]
        wait_fresh: bool,
    },
    /// Fill the login form (#account + #password) and click "登录"; wait until
    /// the LoginPage unmounts. Returns {ok, reason?} so e2e scripts can detect
    /// auth failures (wrong password, locked account) without scraping DOM.
    Login {
        #[arg(long)]
        account: String,
        #[arg(long)]
        password: String,
        /// Max seconds to wait for LoginPage to unmount after submit.
        #[arg(long, default_value = "20")]
        timeout: u64,
    },
    /// Insert text into the Tiptap editor via execCommand.
    TypeMessage { text: String },
    /// Click the send button.
    Send,
    /// Block until the current streaming reply finishes.
    WaitReply {
        #[arg(long, default_value = "30")]
        timeout: u64,
    },
    /// Click the stop button while streaming.
    Cancel {
        /// Wait until chatStore.isStreaming is false (up to 5s) before
        /// returning. Without this flag the command returns as soon as the
        /// click lands, even if the streaming state hasn't unwound yet.
        #[arg(long, default_value_t = false)]
        wait: bool,
    },
    /// Dump all visible messages from the chat as JSON. By default, drops
    /// "empty placeholder" messages (text == "" AND no tool_calls) — these
    /// are the streaming-bubble residue left after a cancelled turn and
    /// carry no assertion value. Use --include-empty to keep them.
    UiMessage {
        /// Only return the last N messages.
        #[arg(long)]
        last: Option<usize>,
        /// Filter by role: user | assistant | tool_call.
        #[arg(long)]
        role: Option<String>,
        /// Only messages rendered within the last N (e.g., "2m", "30s").
        #[arg(long)]
        since: Option<String>,
        /// Include tool_call entries (default true).
        #[arg(long, default_value_t = true)]
        include_tools: bool,
        /// Keep empty-placeholder messages (text=="" && no tool_calls) that
        /// would otherwise be filtered out. Defaults to false.
        #[arg(long, default_value_t = false)]
        include_empty: bool,
    },
    /// Convenience alias for `ui-message --last 1 --role assistant`.
    LastReply {
        #[arg(long)]
        format: Option<String>,
    },
    /// List sidebar conversation rows as JSON.
    ListSessions,
    /// Switch to a conversation by id or index.
    SwitchSession { id_or_index: String },
    /// Archive a conversation by id or index.
    ArchiveSession {
        id_or_index: String,
        /// Wait until list-sessions reflects archived=true for this id (up
        /// to 3s) before returning. The IPC call lands immediately but the
        /// sidebar DOM can take 1-2s to repaint.
        #[arg(long, default_value_t = false)]
        wait: bool,
    },
    /// Switch the active workspace by name.
    SelectWorkspace { name: String },
    /// NOT IMPLEMENTED YET — would sever the pilot socket and needs a
    /// reconnect protocol. Calling this returns `{ok: false}` without
    /// touching the app. Workaround: `goto home` + `goto employees` to
    /// force a store reload, then continue.
    RestartApp,
    /// Dump the current UI state as JSON.
    Where,
    /// Capture a labelled screenshot under /tmp.
    Screenshot {
        #[arg(long)]
        label: String,
        /// CSS selector to limit the capture region. Defaults to
        /// `[data-aijia-message-list]` if present, else `body`. Whole-document
        /// captures usually hit a 30s html-to-image timeout on real workloads.
        #[arg(long)]
        selector: Option<String>,
    },
    /// Verify the app is up and ready for e2e operations.
    HealthCheck,
    /// Archive every conversation whose title starts with the given prefix.
    /// Default prefix is `e2e-test-`. The conversation must be renamed
    /// (e.g. via the UI's rename action) for this filter to match — sending
    /// a message whose text starts with `e2e-test-` does NOT update the title.
    CleanupTestSessions {
        #[arg(long, default_value = "e2e-test-")]
        prefix: String,
    },
    /// Click a top-level sidebar nav entry (data-aijia-nav={page}).
    Goto {
        /// One of: home, employees, expert-teams, skill-center, schedules, channel.
        page: String,
        /// Wait until the corresponding route loads (up to 5s) before returning.
        #[arg(long, default_value_t = false)]
        wait: bool,
    },
    /// Dump tool calls that happened in a turn, as recorded by chatStore.
    /// Defaults to the last turn (the assistant message after the most
    /// recent user message). `--turn N` selects the N-th turn (0-based).
    ToolCalls {
        /// `last` (default) or a 0-based turn index.
        #[arg(long, default_value = "last")]
        turn: String,
    },
    /// Scan currently-rendered tool bubbles in the chat DOM (independent of
    /// `ui-message` filtering). Surfaces UI-level state (spinner / expanded /
    /// result count / first URL / error text) for visual-render assertions.
    ToolBubble {
        /// `last` (default) or a 0-based turn index.
        #[arg(long, default_value = "last")]
        turn: String,
    },
    /// Open the schedules new-agenda editor: click `[data-aijia-agenda-new]`.
    /// Pre: schedules page is active. Post: editor sheet `[data-aijia-agenda-editor]`
    /// is mounted (not waited — chain `agenda-wait-editor` if needed).
    AgendaOpenNew,
    /// Wait until the agenda editor sheet is mounted.
    AgendaWaitEditor {
        #[arg(long, default_value = "5")]
        timeout: u64,
    },
    /// Fill one field inside the open agenda editor.
    /// `--field` ∈ `title` | `prompt` (text inputs).
    AgendaFill {
        #[arg(long)]
        field: String,
        #[arg(long)]
        value: String,
    },
    /// Pick the agenda editor's "频率" select. `--value` ∈
    /// `once` | `daily` | `weekly` | `monthly` | `yearly` (CLI alias maps to
    /// editor's internal `one_shot|daily|...`).
    AgendaSetFrequency {
        #[arg(long)]
        value: String,
    },
    /// Fill the agenda editor's "开始时间" datetime-local input.
    /// `--value` is `YYYY-MM-DDTHH:MM` (matches `datetime-local` browser format).
    AgendaSetStartAt {
        #[arg(long)]
        value: String,
    },
    /// Pick the agenda editor's "执行员工" native select by employee `name`
    /// (substring match against option text — `{avatar} {name} · {role}`).
    ///
    /// CURRENT STATE: AgendaItemEditor does NOT render an employee picker —
    /// `organizerEmployeeId` is only passed in via props, and the backend
    /// silently falls back to `default` when the request omits it. This CLI
    /// will return `employee_select_missing_or_no_employees` until a picker
    /// is added to the UI. Calling it has no effect on the saved agenda.
    AgendaSetEmployee {
        #[arg(long)]
        name: String,
    },
    /// Click the agenda editor's "保存" button. Returns `{ok, disabled?, reason}`.
    /// Does NOT wait for editor to close — chain `agenda-wait-editor-closed`
    /// or `agenda-wait-row` for that.
    AgendaSave,
    /// Click the agenda editor's "取消" button.
    AgendaCancel,
    /// Wait until the agenda editor sheet has unmounted.
    AgendaWaitEditorClosed {
        #[arg(long, default_value = "5")]
        timeout: u64,
    },
    /// Wait for an agenda row with matching `--title` to appear in the list.
    /// Returns `{ok, agendaId, status}` once visible.
    AgendaWaitRow {
        #[arg(long)]
        title: String,
        #[arg(long, default_value = "5")]
        timeout: u64,
    },
    /// Click an action button on a single agenda row identified by `--title`.
    /// `--action` ∈ `run-now` | `pause` | `resume` | `edit` | `cancel` | `restore` | `purge`.
    /// `cancel` / `purge` open a ConfirmDialog — chain `dialog-click --action confirm`
    /// to confirm; this command does NOT auto-confirm.
    AgendaRowAction {
        #[arg(long)]
        title: String,
        #[arg(long)]
        action: String,
    },
    /// Click the named employee card on the employees page.
    /// Pre: employees page is active. Pass exactly one of `--name` (substring
    /// match against `[data-aijia-employee-name]`) or `--id` (exact match
    /// against `[data-aijia-employee-id]`). Use `--id` when multiple employees
    /// share the same name.
    EmployeeOpenCard {
        #[arg(long, conflicts_with = "id")]
        name: Option<String>,
        #[arg(long, conflicts_with = "name")]
        id: Option<String>,
    },
    /// Wait until the employee drawer (`[data-aijia-employee-drawer]`) is mounted.
    EmployeeWaitDrawer {
        #[arg(long, default_value = "3")]
        timeout: u64,
    },
    /// Click the drawer's "现在派活" button (`[data-aijia-employee-action="dispatch"]`).
    /// Pre: drawer is open. Does NOT wait for chat route — caller polls
    /// `where --json` for `sessionId` change.
    EmployeeClickDispatch,
    /// Close the employee drawer (click the close button or the sheet overlay).
    EmployeeCloseDrawer,
    /// Click the sidebar footer "设置" button (`[data-aijia-open-settings]`).
    /// Does NOT wait — chain `settings-wait` for that.
    OpenSettings,
    /// Wait until the settings modal (`[data-aijia-settings-shell]`) is mounted.
    SettingsWait {
        #[arg(long, default_value = "3")]
        timeout: u64,
    },
    /// Click a panel in the settings left menu.
    /// `--key` ∈ `account` | `account-billing` | `archived` | `runtime` | `about`
    /// (matches `SettingsModalKey`; disabled keys like `usage` / `permissions`
    /// won't render).
    SettingsSelectPanel {
        #[arg(long)]
        key: String,
    },
    /// Click the settings modal's close button (`[data-aijia-settings-action="close"]`).
    SettingsClose,
    /// Click the "退出登录" button on the General panel. Pre: settings open
    /// AND panel = `account`. Returns `{ok}` on click; caller waits for
    /// LoginPage to remount via separate probe (e.g. polling `where --json`
    /// for `loggedIn === false`).
    Logout,
    /// Queue absolute file paths so that the next call to `pickAttachments()`
    /// inside the composer returns these paths instead of opening the OS
    /// file dialog. Dev-only — relies on `__aijia._pickAttachmentsMockQueue`.
    /// One queue entry is consumed per `composer-click-plus` click.
    ComposerQueueFiles {
        /// Comma-separated absolute paths (`,` not allowed in filenames).
        #[arg(long)]
        paths: String,
    },
    /// Click the composer's "+" button (`[data-aijia-composer-plus]`).
    /// If a queue entry was set via `composer-queue-files`, the OS dialog
    /// is skipped and the queued paths flow through `makePendingAttachment`
    /// → `insertAttachmentTokens` as if the user had picked them.
    ComposerClickPlus,
    /// Submit the composer by dispatching synthetic `Enter` to the
    /// ProseMirror editor. Required when the visible send button is hidden
    /// (e.g. during streaming the button is replaced by "停止"). The
    /// composer's keydown handler routes Enter → trySubmit; the backend
    /// PendingQueueManager buffers the new message when the current turn
    /// is still streaming. Pre: composer is mounted and not empty.
    ComposerSubmit,
    /// Snapshot the pending-message queue for the active conversation.
    /// Reads `window.__aijia.pendingStore.getState().bySession[sessionId]`
    /// where `sessionId` defaults to `chatStore.activeConversationId`.
    /// Returns `{ok, sessionId, count, items: [{id, source, text,
    /// senderNick, attachments, receivedAt}]}`. Use to verify messages
    /// queued via `composer-submit` during streaming landed in the queue.
    PendingSnapshot {
        /// Override the session id. If omitted, uses
        /// `chatStore.activeConversationId`.
        #[arg(long)]
        session_id: Option<String>,
    },
    /// Click the "雇佣员工" / 卡片市场入口按钮 (`[data-aijia-hire-button]`).
    /// Pre: employees page (or home) is active. Post: HireWizard mounts.
    /// `--variant template-market` 点顶部文字按钮；`--variant add-card` 点
    /// 网格末尾的 `+` 卡片。默认 `template-market`。
    HireOpen {
        #[arg(long, default_value = "template-market")]
        variant: String,
    },
    /// Wait until HireWizard (`[data-aijia-hire-wizard]`) is mounted.
    HireWait {
        #[arg(long, default_value = "3")]
        timeout: u64,
    },
    /// Pick a template card on step 1 by template id or by name (substring).
    /// Post: wizard advances to step 2 automatically.
    HireSelectTemplate {
        /// 模板 id（如 `builtin:xiaoyuan`）。优先按 id 精确匹配。
        #[arg(long, conflicts_with = "name")]
        id: Option<String>,
        /// 模板名（substring 匹配 `data-aijia-hire-template-name`）。
        #[arg(long, conflicts_with = "id")]
        name: Option<String>,
    },
    /// Click `[data-aijia-hire-action="next"]` on step 2.
    /// 若模板 `resourceConfigKind === 'none'` 且无 schema，next === save，
    /// 雇佣直接完成。
    HireNext,
    /// Click `[data-aijia-hire-action="prev"]` on step 2 (returns to step 1).
    HirePrev,
    /// Fill a HireWizard form field. `--field` ∈ `name` | `cron`.
    HireFill {
        #[arg(long)]
        field: String,
        #[arg(long)]
        value: String,
    },
    /// Click `[data-aijia-hire-action="save"]` (alias for `hire-next` 当
    /// step 2 没有 step 3 时；step 3 的 resource-form 用 `resource-save`).
    HireSave,
    /// Read `data-aijia-employee-status` (running | has-report | needs-setup
    /// Read `data-aijia-employee-status` (running | has-report | needs-setup
    /// | idle), `data-aijia-employee-cron-enabled` (true | false | none),
    /// and `data-aijia-employee-dispatch-disabled` (true when employee is
    /// archived) from a card matched by `--name` or `--id` (mutually
    /// exclusive). Use `--id` when multiple employees share the same name.
    EmployeeStatus {
        #[arg(long, conflicts_with = "id")]
        name: Option<String>,
        #[arg(long, conflicts_with = "name")]
        id: Option<String>,
    },
    /// Click a single `[data-aijia-employee-action="<verb>"]` button inside
    /// the open employee drawer. `--action` ∈ `dispatch` | `close` |
    /// `view-chat` | `stop` | `edit-cron` | `toggle-cron` | `toggle-cron-badge` |
    /// `add-cron-trigger` | `config-resource` | `fire`.
    /// `fire` opens a Radix ConfirmDialog — chain `dialog-click --action confirm`.
    EmployeeDrawerAction {
        #[arg(long)]
        action: String,
    },
    /// Click `[data-aijia-employee-action="pause-cron|resume-cron"]` on the
    /// card itself (not inside the drawer). Useful to toggle cron without
    /// opening the drawer first.
    EmployeeCardToggleCron {
        #[arg(long)]
        name: String,
    },
    /// Fill a field in the currently-open ResourceConfigForm (any of the
    /// 5 hand-tuned forms or the SchemaForm). Matches by
    /// `[data-aijia-resource-field="<name>"]`. For MonitoringUrlsForm rows,
    /// pass `--row N` to scope to the N-th row (0-based).
    ResourceFill {
        #[arg(long)]
        field: String,
        #[arg(long)]
        value: String,
        /// 0-based row index (MonitoringUrlsForm only). Omit to target the
        /// first occurrence of the field (suitable for non-row forms).
        #[arg(long)]
        row: Option<usize>,
    },
    /// Click `[data-aijia-resource-action="add-row"]` (MonitoringUrlsForm).
    ResourceAddRow,
    /// Click `[data-aijia-resource-action="remove-row"]` in row N
    /// (MonitoringUrlsForm).
    ResourceRemoveRow {
        #[arg(long)]
        row: usize,
    },
    /// Click `[data-aijia-resource-action="save"]` in the open
    /// ResourceConfigForm.
    ResourceSave,
    /// Click `[data-aijia-resource-action="cancel"]` in the open
    /// ResourceConfigForm.
    ResourceCancel,
    /// Queue a single absolute folder path so the next call to
    /// `pickLocalDirectory()` returns it instead of opening the OS folder
    /// dialog. Dev-only — relies on `__aijia._pickDirectoryMockQueue`.
    /// One queue entry is consumed per `workspace-pick --variant other`.
    /// Downstream `authorizeLocalDirectory` IPC + composer state updates
    /// run on the real path.
    WorkspaceQueuePath {
        #[arg(long)]
        path: String,
    },
    /// Click the home composer's workspace trigger button
    /// (`[data-aijia-workspace-trigger]`) to open the dropdown.
    /// Pre: home page is active (composer is mounted).
    WorkspaceOpenPicker,
    /// Click one item inside the open workspace dropdown.
    /// `--variant` ∈ `default` | `other` | `recent`. `recent` requires
    /// `--path` to identify which recent entry to click. `other` triggers
    /// the OS folder dialog (or consumes a queued mock path in dev).
    WorkspacePick {
        #[arg(long)]
        variant: String,
        /// Required when `--variant recent`: absolute path of the recent
        /// workspace entry. Ignored for `default` / `other`.
        #[arg(long)]
        path: Option<String>,
    },
    /// Click an ExpertTeam card by name to start a new conversation with
    /// that team. Pre: expert-teams page is active. Post: chat route flips
    /// to the newly-created conversation.
    ExpertTeamStart {
        #[arg(long)]
        name: String,
    },
    /// Queue a single absolute path so the next call to skill-center's
    /// `openDialog()` (inside `handleImportDirectory` / `handleImportArchive`)
    /// returns it instead of opening the OS dialog. Dev-only — relies on
    /// `__aijia._pickSkillImportMockQueue`. One queue entry is consumed per
    /// `skill-import-pick` click. The variant (directory vs archive) is
    /// determined by which dropdown item is later clicked, not by the queued
    /// path itself.
    SkillImportQueue {
        /// Absolute folder path (for `--variant directory`) or .zip file
        /// path (for `--variant archive`).
        #[arg(long)]
        path: String,
    },
    /// Click the skill-center "导入技能" dropdown trigger Button
    /// (`[data-aijia-skill-import-trigger]`). Pre: skill-center page is
    /// active. Post: AppDropdown menu opens. Caller must chain
    /// `skill-import-pick --variant ...` to pick the actual import variant.
    SkillImportOpen,
    /// Click one item inside the open skill-import dropdown.
    /// `--variant` ∈ `directory` | `archive`. Selects
    /// `[data-aijia-skill-import-action="<variant>"]` and clicks it.
    /// If a queued mock path was set via `skill-import-queue`, the OS dialog
    /// is skipped and the queued path flows through `runInstall(picked)` →
    /// `installCustomSkill` → backend `install_custom_skill` (which auto
    /// `refresh_skill_registry`s the in-memory store).
    SkillImportPick {
        #[arg(long)]
        variant: String,
    },
    /// Dump skill cards currently rendered on the skill-center page.
    /// Reads `[data-aijia-skill-card]` DOM nodes and returns
    /// `[{id, source, title, version}]`. Use to verify imports landed in
    /// catalog without opening individual cards. Pre: skill-center page
    /// must be active.
    SkillCards,
    /// Read the currently visible dialog (permission-ask / ask-user-question /
    /// confirm) into a JSON snapshot. Queries `[data-aijia-dialog]` and
    /// returns `{kind, tool, title, description, actions}`, or
    /// `{ok: true, dialog: null}` when no dialog is open. Use to inspect
    /// pending dialogs before deciding which `dialog-click --action ...` to
    /// chain.
    DialogSnapshot,
    /// Click one action button inside the currently visible dialog. Waits up
    /// to `--timeout` seconds for the dialog to appear.
    ///
    /// `--action` ∈ `allow` | `deny` | `cancel` | `confirm` | `option`.
    /// For `--action option` in `ask-user-question`, pass `--question-index N`
    /// and `--option-index M` to disambiguate which option button to click.
    DialogClick {
        #[arg(long)]
        action: String,
        /// 0-based question index inside an `ask-user-question` dialog.
        /// Only meaningful for `--action option`.
        #[arg(long)]
        question_index: Option<u32>,
        /// 0-based option index inside the question identified by
        /// `--question-index`. Only meaningful for `--action option`.
        #[arg(long)]
        option_index: Option<u32>,
        #[arg(long, default_value = "10")]
        timeout: u64,
    },
}

#[derive(Subcommand, Debug)]
pub(crate) enum AssertKind {
    /// Assert exact text content
    Text { target: String, expected: String },
    /// Assert element is visible
    Visible { target: String },
    /// Assert element is hidden
    Hidden { target: String },
    /// Assert input value
    Value { target: String, expected: String },
    /// Assert element count matching selector
    Count { selector: String, expected: u64 },
    /// Assert checkbox is checked
    Checked { target: String },
    /// Assert text contains substring
    Contains { target: String, expected: String },
    /// Assert current URL contains string
    Url { expected: String },
}

#[derive(clap::Args, Debug)]
pub(crate) struct StorageArgs {
    /// Use sessionStorage instead of localStorage.
    #[arg(long)]
    pub session: bool,
    #[command(subcommand)]
    pub action: StorageAction,
}

#[derive(clap::Args, Debug)]
pub(crate) struct FormsArgs {
    /// Target a specific form by CSS selector.
    #[arg(long)]
    pub selector: Option<String>,
}

#[derive(Subcommand, Debug)]
pub(crate) enum StorageAction {
    /// Get a value by key.
    Get { key: String },
    /// Set a key-value pair.
    Set { key: String, value: String },
    /// List all key-value pairs.
    List,
    /// Clear all storage.
    Clear,
}

#[derive(Subcommand, Debug)]
pub(crate) enum RecordAction {
    /// Start recording interactions
    Start,
    /// Stop recording and save to file
    Stop {
        /// Output file path (JSON)
        #[arg(short, long)]
        output: PathBuf,
    },
    /// Show recording status
    Status,
}

/// Parsed target for element-targeting commands.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Target {
    Ref(String),
    Selector(String),
    Coords(i32, i32),
}

/// Parse a target string into a `Target` variant.
pub(crate) fn parse_target(s: &str) -> Target {
    if let Some(r) = s.strip_prefix('@') {
        return Target::Ref(r.to_owned());
    }

    if let Some((x_str, y_str)) = s.split_once(',')
        && let (Ok(x), Ok(y)) = (x_str.trim().parse::<i32>(), y_str.trim().parse::<i32>())
    {
        return Target::Coords(x, y);
    }

    Target::Selector(s.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_target_ref() {
        assert_eq!(parse_target("@e1"), Target::Ref("e1".to_owned()));
        assert_eq!(parse_target("@e42"), Target::Ref("e42".to_owned()));
    }

    #[test]
    fn test_parse_target_selector() {
        assert_eq!(
            parse_target("#submit-btn"),
            Target::Selector("#submit-btn".to_owned())
        );
        assert_eq!(
            parse_target(".class"),
            Target::Selector(".class".to_owned())
        );
    }

    #[test]
    fn test_parse_target_coords() {
        assert_eq!(parse_target("100,200"), Target::Coords(100, 200));
        assert_eq!(parse_target("0, 0"), Target::Coords(0, 0));
    }

    #[test]
    fn test_parse_target_invalid_coords_as_selector() {
        assert_eq!(
            parse_target("abc,def"),
            Target::Selector("abc,def".to_owned())
        );
    }

    #[test]
    fn test_parse_diff_command() {
        let cli = Cli::parse_from(["tauri-pilot", "--socket", "/tmp/test.sock", "diff"]);
        assert!(matches!(
            cli.command,
            Command::Diff {
                r#ref: None,
                interactive: false,
                selector: None,
                depth: None,
            }
        ));
    }

    #[test]
    fn test_parse_diff_with_ref() {
        let cli = Cli::parse_from([
            "tauri-pilot",
            "--socket",
            "/tmp/test.sock",
            "diff",
            "--ref",
            "/tmp/snap.json",
        ]);
        if let Command::Diff {
            r#ref: Some(path), ..
        } = cli.command
        {
            assert_eq!(path, std::path::PathBuf::from("/tmp/snap.json"));
        } else {
            panic!("Expected Diff command with ref");
        }
    }

    #[test]
    fn test_parse_assert_text() {
        let cli = Cli::parse_from([
            "tauri-pilot",
            "--socket",
            "/tmp/test.sock",
            "assert",
            "text",
            "@e1",
            "Dashboard",
        ]);
        if let Command::Assert(AssertKind::Text { target, expected }) = cli.command {
            assert_eq!(target, "@e1");
            assert_eq!(expected, "Dashboard");
        } else {
            panic!("Expected Assert Text command");
        }
    }

    #[test]
    fn test_parse_assert_visible() {
        let cli = Cli::parse_from([
            "tauri-pilot",
            "--socket",
            "/tmp/test.sock",
            "assert",
            "visible",
            "#submit",
        ]);
        if let Command::Assert(AssertKind::Visible { target }) = cli.command {
            assert_eq!(target, "#submit");
        } else {
            panic!("Expected Assert Visible command");
        }
    }

    #[test]
    fn test_parse_assert_count() {
        let cli = Cli::parse_from([
            "tauri-pilot",
            "--socket",
            "/tmp/test.sock",
            "assert",
            "count",
            ".list-item",
            "5",
        ]);
        if let Command::Assert(AssertKind::Count { selector, expected }) = cli.command {
            assert_eq!(selector, ".list-item");
            assert_eq!(expected, 5);
        } else {
            panic!("Expected Assert Count command");
        }
    }

    #[test]
    fn test_parse_assert_url() {
        let cli = Cli::parse_from([
            "tauri-pilot",
            "--socket",
            "/tmp/test.sock",
            "assert",
            "url",
            "/dashboard",
        ]);
        if let Command::Assert(AssertKind::Url { expected }) = cli.command {
            assert_eq!(expected, "/dashboard");
        } else {
            panic!("Expected Assert Url command");
        }
    }

    #[test]
    fn test_parse_watch_command() {
        let cli = Cli::parse_from([
            "tauri-pilot",
            "--socket",
            "/tmp/test.sock",
            "watch",
            "--selector",
            ".results",
            "--timeout",
            "5000",
            "--stable",
            "500",
        ]);
        if let Command::Watch {
            selector,
            timeout,
            stable,
            require_mutation,
        } = cli.command
        {
            assert_eq!(selector, Some(".results".to_owned()));
            assert_eq!(timeout, 5000);
            assert_eq!(stable, 500);
            assert!(!require_mutation);
        } else {
            panic!("Expected Watch command");
        }
    }

    #[test]
    fn test_parse_watch_defaults() {
        let cli = Cli::parse_from(["tauri-pilot", "--socket", "/tmp/test.sock", "watch"]);
        if let Command::Watch {
            selector,
            timeout,
            stable,
            require_mutation,
        } = cli.command
        {
            assert_eq!(selector, None);
            assert_eq!(timeout, 10000);
            assert_eq!(stable, 300);
            assert!(!require_mutation);
        } else {
            panic!("Expected Watch command");
        }
    }

    #[test]
    fn test_parse_watch_require_mutation() {
        let cli = Cli::parse_from([
            "tauri-pilot",
            "--socket",
            "/tmp/test.sock",
            "watch",
            "--require-mutation",
        ]);
        if let Command::Watch {
            require_mutation, ..
        } = cli.command
        {
            assert!(require_mutation);
        } else {
            panic!("Expected Watch command");
        }
    }

    #[test]
    fn test_parse_watch_require_mutation_with_selector() {
        let cli = Cli::parse_from([
            "tauri-pilot",
            "--socket",
            "/tmp/test.sock",
            "watch",
            "--selector",
            "#root",
            "--require-mutation",
            "--stable",
            "500",
        ]);
        if let Command::Watch {
            selector,
            stable,
            require_mutation,
            ..
        } = cli.command
        {
            assert_eq!(selector, Some("#root".to_owned()));
            assert_eq!(stable, 500);
            assert!(require_mutation);
        } else {
            panic!("Expected Watch command");
        }
    }

    #[test]
    fn test_parse_drag_to_element() {
        let cli = Cli::parse_from([
            "tauri-pilot",
            "--socket",
            "/tmp/t.sock",
            "drag",
            "@e5",
            "@e6",
        ]);
        assert!(
            matches!(cli.command, Command::Drag { ref source, target: Some(_), .. } if source == "@e5")
        );
    }

    #[test]
    fn test_parse_drag_with_offset() {
        let cli = Cli::parse_from([
            "tauri-pilot",
            "--socket",
            "/tmp/t.sock",
            "drag",
            "@e5",
            "--offset",
            "0,100",
        ]);
        assert!(
            matches!(cli.command, Command::Drag { ref source, offset: Some(ref off), .. } if source == "@e5" && off == "0,100")
        );
    }

    #[test]
    fn test_parse_drop_with_file() {
        let cli = Cli::parse_from([
            "tauri-pilot",
            "--socket",
            "/tmp/t.sock",
            "drop",
            "@e3",
            "--file",
            "test.png",
        ]);
        assert!(matches!(cli.command, Command::Drop { ref target, .. } if target == "@e3"));
    }

    #[test]
    fn test_parse_drop_multiple_files() {
        let cli = Cli::parse_from([
            "tauri-pilot",
            "--socket",
            "/tmp/t.sock",
            "drop",
            "@e3",
            "--file",
            "a.png",
            "--file",
            "b.txt",
        ]);
        if let Command::Drop { file, .. } = cli.command {
            assert_eq!(file.len(), 2);
        } else {
            panic!("expected Drop command");
        }
    }

    #[test]
    fn test_parse_drag_rejects_both_target_and_offset() {
        let result = Cli::try_parse_from([
            "tauri-pilot",
            "--socket",
            "/tmp/t.sock",
            "drag",
            "@e5",
            "@e6",
            "--offset",
            "0,100",
        ]);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_drop_requires_file() {
        let result = Cli::try_parse_from(["tauri-pilot", "--socket", "/tmp/t.sock", "drop", "@e3"]);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_storage_get() {
        let cli = Cli::parse_from([
            "tauri-pilot",
            "--socket",
            "/tmp/t.sock",
            "storage",
            "get",
            "auth_token",
        ]);
        if let Command::Storage(StorageArgs {
            session,
            action: StorageAction::Get { key },
        }) = cli.command
        {
            assert!(!session);
            assert_eq!(key, "auth_token");
        } else {
            panic!("Expected Storage Get command");
        }
    }

    #[test]
    fn test_parse_storage_set() {
        let cli = Cli::parse_from([
            "tauri-pilot",
            "--socket",
            "/tmp/t.sock",
            "storage",
            "set",
            "theme",
            "dark",
        ]);
        if let Command::Storage(StorageArgs {
            session,
            action: StorageAction::Set { key, value },
        }) = cli.command
        {
            assert!(!session);
            assert_eq!(key, "theme");
            assert_eq!(value, "dark");
        } else {
            panic!("Expected Storage Set command");
        }
    }

    #[test]
    fn test_parse_storage_list_session() {
        let cli = Cli::parse_from([
            "tauri-pilot",
            "--socket",
            "/tmp/t.sock",
            "storage",
            "--session",
            "list",
        ]);
        if let Command::Storage(StorageArgs {
            session,
            action: StorageAction::List,
        }) = cli.command
        {
            assert!(session);
        } else {
            panic!("Expected Storage List command with session flag");
        }
    }

    #[test]
    fn test_parse_storage_clear() {
        let cli = Cli::parse_from(["tauri-pilot", "--socket", "/tmp/t.sock", "storage", "clear"]);
        assert!(matches!(
            cli.command,
            Command::Storage(StorageArgs {
                action: StorageAction::Clear,
                ..
            })
        ));
    }

    #[test]
    fn test_parse_forms_command() {
        let cli = Cli::parse_from(["tauri-pilot", "--socket", "/tmp/test.sock", "forms"]);
        if let Command::Forms(FormsArgs { selector }) = cli.command {
            assert_eq!(selector, None);
        } else {
            panic!("Expected Forms command");
        }
    }

    #[test]
    fn test_parse_forms_with_selector() {
        let cli = Cli::parse_from([
            "tauri-pilot",
            "--socket",
            "/tmp/test.sock",
            "forms",
            "--selector",
            "#login",
        ]);
        if let Command::Forms(FormsArgs { selector }) = cli.command {
            assert_eq!(selector, Some("#login".to_owned()));
        } else {
            panic!("Expected Forms command with selector");
        }
    }

    #[test]
    fn test_parse_record_start() {
        let cli = Cli::parse_from([
            "tauri-pilot",
            "--socket",
            "/tmp/test.sock",
            "record",
            "start",
        ]);
        assert!(matches!(
            cli.command,
            Command::Record {
                action: RecordAction::Start
            }
        ));
    }

    #[test]
    fn test_parse_record_stop_with_output() {
        let cli = Cli::parse_from([
            "tauri-pilot",
            "--socket",
            "/tmp/test.sock",
            "record",
            "stop",
            "--output",
            "test.json",
        ]);
        if let Command::Record {
            action: RecordAction::Stop { output },
        } = cli.command
        {
            assert_eq!(output, std::path::PathBuf::from("test.json"));
        } else {
            panic!("Expected Record Stop command with output");
        }
    }

    #[test]
    fn test_parse_record_stop_requires_output() {
        let result = Cli::try_parse_from([
            "tauri-pilot",
            "--socket",
            "/tmp/test.sock",
            "record",
            "stop",
        ]);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_replay_with_export() {
        let cli = Cli::parse_from([
            "tauri-pilot",
            "--socket",
            "/tmp/test.sock",
            "replay",
            "test.json",
            "--export",
            "sh",
        ]);
        if let Command::Replay { path, export } = cli.command {
            assert_eq!(path, std::path::PathBuf::from("test.json"));
            assert_eq!(export, Some("sh".to_owned()));
        } else {
            panic!("Expected Replay command with export");
        }
    }

    #[test]
    fn test_windows_command() {
        let cli = Cli::parse_from(["tauri-pilot", "--socket", "/tmp/test.sock", "windows"]);
        assert!(matches!(cli.command, Command::Windows));
    }

    #[test]
    fn test_mcp_command() {
        let cli = Cli::parse_from(["tauri-pilot", "mcp"]);
        assert!(matches!(cli.command, Command::Mcp));
    }

    #[test]
    fn test_window_flag_with_command() {
        let cli = Cli::parse_from([
            "tauri-pilot",
            "--socket",
            "/tmp/test.sock",
            "--window",
            "settings",
            "snapshot",
        ]);
        assert_eq!(cli.window, Some("settings".to_owned()));
        assert!(matches!(cli.command, Command::Snapshot { .. }));
    }

    #[test]
    #[serial_test::serial]
    fn test_window_flag_env() {
        // SAFETY: test is serialized via #[serial] to prevent parallel env access
        unsafe {
            std::env::set_var("TAURI_PILOT_WINDOW", "main");
        }
        let cli = Cli::parse_from(["tauri-pilot", "--socket", "/tmp/test.sock", "ping"]);
        assert_eq!(cli.window, Some("main".to_owned()));
        unsafe {
            std::env::remove_var("TAURI_PILOT_WINDOW");
        }
    }

    #[test]
    fn test_parse_eval_with_script_containing_quotes() {
        // Scripts with double quotes are the primary motivation for stdin support —
        // the shell would mangle them if passed as a CLI argument.
        let script = r#"document.querySelector('[data-id="main"]').textContent"#;
        let cli = Cli::parse_from(["tauri-pilot", "--socket", "/tmp/test.sock", "eval", script]);
        if let Command::Eval { script: parsed } = cli.command {
            assert_eq!(parsed, Some(script.to_owned()));
        } else {
            panic!("Expected Eval command");
        }
    }

    #[test]
    fn test_parse_eval_dash_reads_stdin() {
        let cli = Cli::parse_from(["tauri-pilot", "--socket", "/tmp/test.sock", "eval", "-"]);
        if let Command::Eval { script } = cli.command {
            assert_eq!(script, Some("-".to_owned()));
        } else {
            panic!("Expected Eval command with dash");
        }
    }

    #[test]
    fn test_parse_eval_no_arg_reads_stdin() {
        let cli = Cli::parse_from(["tauri-pilot", "--socket", "/tmp/test.sock", "eval"]);
        if let Command::Eval { script } = cli.command {
            assert_eq!(script, None);
        } else {
            panic!("Expected Eval command with no script");
        }
    }

    #[test]
    fn test_parse_snapshot_with_save() {
        let cli = Cli::parse_from([
            "tauri-pilot",
            "--socket",
            "/tmp/test.sock",
            "snapshot",
            "--save",
            "/tmp/snap.json",
        ]);
        if let Command::Snapshot {
            save: Some(path), ..
        } = cli.command
        {
            assert_eq!(path, std::path::PathBuf::from("/tmp/snap.json"));
        } else {
            panic!("Expected Snapshot command with save");
        }
    }

    #[test]
    fn test_parse_aijia_new_task() {
        let cli = Cli::parse_from(["tauri-pilot", "aijia", "new-task"]);
        assert!(matches!(
            cli.command,
            Command::Aijia {
                command: AijiaCommand::NewTask { wait_fresh: false }
            }
        ));
    }

    #[test]
    fn test_parse_aijia_new_task_wait_fresh() {
        let cli = Cli::parse_from(["tauri-pilot", "aijia", "new-task", "--wait-fresh"]);
        assert!(matches!(
            cli.command,
            Command::Aijia {
                command: AijiaCommand::NewTask { wait_fresh: true }
            }
        ));
    }

    #[test]
    fn test_parse_aijia_type_message() {
        let cli = Cli::parse_from(["tauri-pilot", "aijia", "type-message", "你好"]);
        if let Command::Aijia {
            command: AijiaCommand::TypeMessage { text },
        } = cli.command
        {
            assert_eq!(text, "你好");
        } else {
            panic!("Expected aijia type-message command");
        }
    }

    #[test]
    fn test_parse_aijia_wait_reply_default_timeout() {
        let cli = Cli::parse_from(["tauri-pilot", "aijia", "wait-reply"]);
        if let Command::Aijia {
            command: AijiaCommand::WaitReply { timeout },
        } = cli.command
        {
            assert_eq!(timeout, 30);
        } else {
            panic!("Expected aijia wait-reply command");
        }
    }

    #[test]
    fn test_parse_aijia_ui_message_with_filters() {
        let cli = Cli::parse_from([
            "tauri-pilot",
            "aijia",
            "ui-message",
            "--last",
            "3",
            "--role",
            "assistant",
        ]);
        if let Command::Aijia {
            command:
                AijiaCommand::UiMessage {
                    last,
                    role,
                    include_tools,
                    ..
                },
        } = cli.command
        {
            assert_eq!(last, Some(3));
            assert_eq!(role.as_deref(), Some("assistant"));
            assert!(include_tools);
        } else {
            panic!("Expected aijia ui-message command");
        }
    }

    #[test]
    fn test_parse_aijia_screenshot_requires_label() {
        let result = Cli::try_parse_from(["tauri-pilot", "aijia", "screenshot"]);
        assert!(result.is_err(), "--label is required");
    }

    #[test]
    fn test_parse_aijia_screenshot_with_label() {
        let cli = Cli::parse_from([
            "tauri-pilot",
            "aijia",
            "screenshot",
            "--label",
            "post-send",
        ]);
        if let Command::Aijia {
            command: AijiaCommand::Screenshot { label, selector },
        } = cli.command
        {
            assert_eq!(label, "post-send");
            assert_eq!(selector, None);
        } else {
            panic!("Expected aijia screenshot command");
        }
    }

    #[test]
    fn test_parse_aijia_screenshot_with_selector() {
        let cli = Cli::parse_from([
            "tauri-pilot",
            "aijia",
            "screenshot",
            "--label",
            "msg-area",
            "--selector",
            "[data-aijia-message-list]",
        ]);
        if let Command::Aijia {
            command: AijiaCommand::Screenshot { label, selector },
        } = cli.command
        {
            assert_eq!(label, "msg-area");
            assert_eq!(selector.as_deref(), Some("[data-aijia-message-list]"));
        } else {
            panic!("Expected aijia screenshot command with selector");
        }
    }
}
