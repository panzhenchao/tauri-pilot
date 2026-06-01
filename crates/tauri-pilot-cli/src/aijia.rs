//! AIjia-specific subcommands for end-to-end testing of the AIjia desktop app.
//!
//! All operations against the AIjia webview funnel through these subcommands —
//! e2e scripts MUST NOT call generic `tauri-pilot eval/click/snapshot` directly.
//! See `lotus-app/docs/e2e-org1-chat-mainline.md` for the rule.
//!
//! The dev build of AIjia exposes `window.__aijia.chatStore` /
//! `window.__aijia.sessionStore` (Zustand hooks). Reading state via
//! `store.getState()` is faster and more accurate than scraping the DOM, so
//! most commands here route through the store first and fall back to DOM
//! selectors (`[data-aijia-*]`) only when state isn't available.

use anyhow::{Context, Result};
use serde_json::{Value, json};
use std::time::{Duration, Instant};

use crate::cli::AijiaCommand;
use crate::client::Client;
use crate::with_window;

pub(crate) async fn dispatch(
    client: &mut Client,
    cmd: AijiaCommand,
    window: Option<&str>,
) -> Result<Value> {
    match cmd {
        AijiaCommand::HealthCheck => health_check(client, window).await,
        AijiaCommand::Where => where_state(client, window).await,
        AijiaCommand::Login { account, password, timeout } => {
            login(client, &account, &password, timeout, window).await
        }
        AijiaCommand::NewTask { wait_fresh } => new_task(client, wait_fresh, window).await,
        AijiaCommand::TypeMessage { text } => type_message(client, &text, window).await,
        AijiaCommand::Send => send(client, window).await,
        AijiaCommand::Cancel { wait } => cancel(client, wait, window).await,
        AijiaCommand::WaitReply { timeout } => wait_reply(client, timeout, window).await,
        AijiaCommand::UiMessage {
            last,
            role,
            since,
            include_tools,
            include_empty,
        } => {
            ui_message(
                client,
                last,
                role.as_deref(),
                since.as_deref(),
                include_tools,
                include_empty,
                window,
            )
            .await
        }
        AijiaCommand::LastReply { format: _ } => last_reply(client, window).await,
        AijiaCommand::ListSessions => list_sessions(client, window).await,
        AijiaCommand::SwitchSession { id_or_index } => {
            switch_session(client, &id_or_index, window).await
        }
        AijiaCommand::ArchiveSession { id_or_index, wait } => {
            archive_session(client, &id_or_index, wait, window).await
        }
        AijiaCommand::SelectWorkspace { name } => select_workspace(client, &name, window).await,
        AijiaCommand::RestartApp => restart_app(client, window).await,
        AijiaCommand::Screenshot { label, selector } => {
            screenshot(client, &label, selector.as_deref(), window).await
        }
        AijiaCommand::CleanupTestSessions { prefix } => {
            cleanup_test_sessions(client, &prefix, window).await
        }
        AijiaCommand::Goto { page, wait } => goto(client, &page, wait, window).await,
        AijiaCommand::ToolCalls { turn } => tool_calls(client, &turn, window).await,
        AijiaCommand::ToolBubble { turn } => tool_bubble(client, &turn, window).await,
        AijiaCommand::AgendaOpenNew => agenda_open_new(client, window).await,
        AijiaCommand::AgendaWaitEditor { timeout } => {
            agenda_wait_editor(client, timeout, true, window).await
        }
        AijiaCommand::AgendaWaitEditorClosed { timeout } => {
            agenda_wait_editor(client, timeout, false, window).await
        }
        AijiaCommand::AgendaFill { field, value } => {
            agenda_fill(client, &field, &value, window).await
        }
        AijiaCommand::AgendaSetFrequency { value } => {
            agenda_set_frequency(client, &value, window).await
        }
        AijiaCommand::AgendaSetStartAt { value } => {
            agenda_set_start_at(client, &value, window).await
        }
        AijiaCommand::AgendaSetEmployee { name } => {
            agenda_set_employee(client, &name, window).await
        }
        AijiaCommand::AgendaSave => agenda_editor_action(client, "save", window).await,
        AijiaCommand::AgendaCancel => agenda_editor_action(client, "cancel", window).await,
        AijiaCommand::AgendaWaitRow { title, timeout } => {
            agenda_wait_row(client, &title, timeout, window).await
        }
        AijiaCommand::AgendaRowAction { title, action } => {
            agenda_row_action(client, &title, &action, window).await
        }
        AijiaCommand::EmployeeOpenCard { name, id } => {
            employee_open_card(client, name.as_deref(), id.as_deref(), window).await
        }
        AijiaCommand::EmployeeWaitDrawer { timeout } => {
            employee_wait_drawer(client, timeout, window).await
        }
        AijiaCommand::EmployeeClickDispatch => employee_click_dispatch(client, window).await,
        AijiaCommand::EmployeeCloseDrawer => employee_close_drawer(client, window).await,
        AijiaCommand::OpenSettings => open_settings(client, window).await,
        AijiaCommand::SettingsWait { timeout } => settings_wait(client, timeout, window).await,
        AijiaCommand::SettingsSelectPanel { key } => {
            settings_select_panel(client, &key, window).await
        }
        AijiaCommand::SettingsClose => settings_close(client, window).await,
        AijiaCommand::Logout => logout(client, window).await,
        AijiaCommand::ComposerQueueFiles { paths } => {
            composer_queue_files(client, &paths, window).await
        }
        AijiaCommand::ComposerClickPlus => composer_click_plus(client, window).await,
        AijiaCommand::HireOpen { variant } => hire_open(client, &variant, window).await,
        AijiaCommand::HireWait { timeout } => hire_wait(client, timeout, window).await,
        AijiaCommand::HireSelectTemplate { id, name } => {
            hire_select_template(client, id.as_deref(), name.as_deref(), window).await
        }
        AijiaCommand::HireNext => hire_action(client, "next", window).await,
        AijiaCommand::HirePrev => hire_action(client, "prev", window).await,
        AijiaCommand::HireSave => hire_action(client, "save", window).await,
        AijiaCommand::HireFill { field, value } => {
            hire_fill(client, &field, &value, window).await
        }
        AijiaCommand::EmployeeStatus { name, id } => {
            employee_status(client, name.as_deref(), id.as_deref(), window).await
        }
        AijiaCommand::EmployeeDrawerAction { action } => {
            employee_drawer_action(client, &action, window).await
        }
        AijiaCommand::EmployeeCardToggleCron { name } => {
            employee_card_toggle_cron(client, &name, window).await
        }
        AijiaCommand::ResourceFill { field, value, row } => {
            resource_fill(client, &field, &value, row, window).await
        }
        AijiaCommand::ResourceAddRow => resource_action(client, "add-row", None, window).await,
        AijiaCommand::ResourceRemoveRow { row } => {
            resource_action(client, "remove-row", Some(row), window).await
        }
        AijiaCommand::ResourceSave => resource_action(client, "save", None, window).await,
        AijiaCommand::ResourceCancel => resource_action(client, "cancel", None, window).await,
        AijiaCommand::WorkspaceQueuePath { path } => {
            workspace_queue_path(client, &path, window).await
        }
        AijiaCommand::WorkspaceOpenPicker => workspace_open_picker(client, window).await,
        AijiaCommand::WorkspacePick { variant, path } => {
            workspace_pick(client, &variant, path.as_deref(), window).await
        }
        AijiaCommand::ExpertTeamStart { name } => expert_team_start(client, &name, window).await,
        AijiaCommand::SkillImportQueue { path } => {
            skill_import_queue(client, &path, window).await
        }
        AijiaCommand::SkillImportOpen => skill_import_open(client, window).await,
        AijiaCommand::SkillImportPick { variant } => {
            skill_import_pick(client, &variant, window).await
        }
        AijiaCommand::SkillCards => skill_cards(client, window).await,
        AijiaCommand::DialogSnapshot => dialog_snapshot(client, window).await,
        AijiaCommand::DialogClick {
            action,
            question_index,
            option_index,
            timeout,
        } => dialog_click(client, &action, question_index, option_index, timeout, window).await,
    }
}

// ─── eval helpers ────────────────────────────────────────────────────────────

/// Radix DropdownMenuTrigger / SelectTrigger / AppDropdown trigger 都监听
/// `onPointerDown` 而不是 `click` —— 单纯 `el.click()` 不会打开 dropdown。
/// 这段 JS 合成完整的 PointerEvent → MouseEvent 序列。
const RADIX_DROPDOWN_CLICK_HELPER_JS: &str = r#"
function __aijia_radixDropdownClick(el) {
    const rect = el.getBoundingClientRect();
    const x = rect.left + rect.width / 2;
    const y = rect.top + rect.height / 2;
    const base = {
        bubbles: true,
        cancelable: true,
        composed: true,
        clientX: x,
        clientY: y,
        screenX: x,
        screenY: y,
        button: 0,
        buttons: 1,
    };
    const pointerOpts = Object.assign({}, base, {
        pointerId: 1,
        pointerType: 'mouse',
        isPrimary: true,
    });
    el.dispatchEvent(new PointerEvent('pointerdown', pointerOpts));
    el.dispatchEvent(new MouseEvent('mousedown', base));
    el.dispatchEvent(new PointerEvent('pointerup', pointerOpts));
    el.dispatchEvent(new MouseEvent('mouseup', base));
    el.dispatchEvent(new MouseEvent('click', base));
}
"#;

async fn eval_json(client: &mut Client, script: &str, window: Option<&str>) -> Result<Value> {
    client
        .call("eval", with_window(Some(json!({ "script": script })), window))
        .await
}

/// Wrap user-provided text into a single-quoted JS string literal, with
/// backslash + single-quote + newline escaping. Suitable for direct injection
/// into an eval'd script.
fn js_string_literal(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('\'');
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '\'' => out.push_str("\\'"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            // U+2028 / U+2029 break JS string literals; encode as \uXXXX.
            '\u{2028}' => out.push_str("\\u2028"),
            '\u{2029}' => out.push_str("\\u2029"),
            other => out.push(other),
        }
    }
    out.push('\'');
    out
}

// ─── health-check + where ────────────────────────────────────────────────────

async fn health_check(client: &mut Client, window: Option<&str>) -> Result<Value> {
    // 1. pilot socket is responsive
    if client
        .call("ping", with_window(None, window))
        .await
        .is_err()
    {
        return Ok(json!({"ok": false, "reason": "pilot ping failed"}));
    }

    // 2. webview document.readyState
    let state = client
        .call("state", with_window(None, window))
        .await
        .context("state call")?;
    let ready = state
        .get("readyState")
        .and_then(Value::as_str)
        .unwrap_or("");
    if ready != "complete" {
        return Ok(json!({
            "ok": false,
            "reason": "document not ready",
            "readyState": ready,
        }));
    }

    // 3. dev-mode store is exposed and active conversation has an id
    let probe = eval_json(
        client,
        r"(() => {
            const a = window.__aijia;
            if (!a || !a.chatStore || !a.sessionStore) return {ok:false, reason:'window.__aijia missing'};
            const cs = a.chatStore.getState();
            return {ok:true, activeConversationId: cs.activeConversationId, hasEditor: !!document.querySelector('.ProseMirror')};
        })()",
        window,
    )
    .await?;

    if probe.get("ok").and_then(Value::as_bool) != Some(true) {
        return Ok(probe);
    }

    Ok(json!({
        "ok": true,
        "readyState": ready,
        "activeConversationId": probe.get("activeConversationId").cloned().unwrap_or(Value::Null),
        "hasEditor": probe.get("hasEditor").cloned().unwrap_or(Value::Bool(false)),
    }))
}

async fn where_state(client: &mut Client, window: Option<&str>) -> Result<Value> {
    // 不再 bridge `state` 拿 url / title：lotus-app 是 SPA 单页应用，
    // window.location 恒为 tauri://localhost/，document.title 恒为
    // index.html 写死的 "AI小家 — 你的智能工作助手"——两者跟"用户在哪个面板"
    // 毫无关系，留在输出里会误导 caller。真路由 = uiStore.route.kind。

    let store = eval_json(
        client,
        r"(() => {
            const a = window.__aijia;
            if (!a || !a.chatStore) return null;
            const cs = a.chatStore.getState();
            const activeId = cs.activeConversationId;
            const stream = activeId ? cs.streamStates?.[activeId] : null;
            const conv = cs.conversations?.find(c => c.id === activeId);
            // Auth-derived scope: `t_{tenantId}__u_{userId}` matches the
            // on-disk users/ partition. Returns null when not logged in.
            const auth = a.authStore?.getState?.();
            const tenantId = auth?.tenant?.id ?? null;
            const userId = auth?.user?.id ?? null;
            const scope = (tenantId != null && userId != null)
                ? ('t_' + tenantId + '__u_' + userId)
                : null;
            return {
                sessionId: activeId,
                sessionName: conv?.title ?? null,
                isStreaming: !!cs.isStreaming,
                isSending: !!stream?.isSending,
                hasToolCallBlock: !!(stream?.toolExecutions || cs.toolExecutions || []).some(t => t.status === 'executing'),
                messageCount: (cs.messages || []).length,
                lastError: stream?.lastError ?? null,
                scope: scope,
                tenantId: tenantId,
                userId: userId,
                loggedIn: !!auth?.isLoggedIn,
            };
        })()",
        window,
    )
    .await?;

    // route 来自 zustand uiStore.route（tagged union {kind, ...payload}）。
    // 单 string `kind` 给 caller 用作 page 判断；完整 object 留在 `routeObj`
    // 让需要 chat conversationId / channel sessionId 的 caller 也能拿到。
    let route = eval_json(
        client,
        r"(() => {
            const ui = window.__aijia?.uiStore?.getState?.();
            if (!ui || !ui.route || typeof ui.route.kind !== 'string') return null;
            return ui.route;
        })()",
        window,
    )
    .await
    .unwrap_or(Value::Null);
    let route_kind = route
        .get("kind")
        .cloned()
        .unwrap_or(Value::Null);

    let has_editor = eval_json(
        client,
        "(() => !!document.querySelector('.ProseMirror'))()",
        window,
    )
    .await
    .unwrap_or(Value::Bool(false));

    let mut out = serde_json::Map::new();
    out.insert("route".into(), route_kind);
    out.insert("routeObj".into(), route);
    if let Value::Object(map) = store {
        for (k, v) in map {
            out.insert(k, v);
        }
    }
    out.insert("hasEditor".into(), has_editor);
    // workspace / model: not exposed via store today; surface explicit null so
    // callers can spot "field exists, value unknown" vs "field missing entirely".
    out.entry("workspace".to_string())
        .or_insert(Value::Null);
    out.entry("model".to_string()).or_insert(Value::Null);

    Ok(Value::Object(out))
}

// ─── auth: login ─────────────────────────────────────────────────────────────

async fn login(
    client: &mut Client,
    account: &str,
    password: &str,
    timeout: u64,
    window: Option<&str>,
) -> Result<Value> {
    // LoginPage (`src/components/auth/LoginPage.tsx`) renders #account + #password
    // inputs and a submit Button labelled `login.loginButtonLabel` = "登录".
    //
    // Ready signal: poll `authStore.{isLoggedIn, isAuthPending}` further below —
    // NOT "LoginPage unmounted", because AuthGate shows <FullscreenLoader/>
    // mid-request which also unmounts #account and would race a naive DOM probe.
    //
    // The inputs are React-controlled — value setter must go through the
    // native prototype setter and dispatch an `input` event, otherwise React
    // overwrites our value on the next render.
    let acct_lit = js_string_literal(account);
    let pwd_lit = js_string_literal(password);
    let submit_script = format!(
        r#"(() => {{
            const acct = document.querySelector('#account');
            const pwd = document.querySelector('#password');
            if (!acct || !pwd) return {{ok: false, reason: 'login_form_not_found'}};

            const proto = window.HTMLInputElement.prototype;
            const setter = Object.getOwnPropertyDescriptor(proto, 'value').set;

            setter.call(acct, {acct});
            acct.dispatchEvent(new Event('input', {{bubbles: true}}));
            setter.call(pwd, {pwd});
            pwd.dispatchEvent(new Event('input', {{bubbles: true}}));

            // Scope the submit button lookup to the form that owns #account,
            // so a stray type="submit" elsewhere on the page can't be picked.
            const form = acct.closest('form');
            if (!form) return {{ok: false, reason: 'login_form_root_not_found'}};
            const btn = [...form.querySelectorAll('button[type="submit"]')]
                .find(b => {{
                    const t = (b.textContent || '').trim();
                    return t === '登录' || t === '登录中…';
                }});
            if (!btn) return {{ok: false, reason: 'login_button_not_found'}};
            if (btn.disabled) return {{ok: false, reason: 'login_button_disabled'}};
            btn.click();
            return {{ok: true}};
        }})()"#,
        acct = acct_lit,
        pwd = pwd_lit,
    );
    let click = eval_json(client, &submit_script, window).await?;
    if click.get("ok").and_then(Value::as_bool) != Some(true) {
        return Ok(click);
    }

    // Poll authStore directly — DOM-based ready signals are unreliable
    // because AuthGate renders <FullscreenLoader /> (which unmounts
    // LoginPage and #account) during isAuthPending. A naive "#account
    // disappeared = logged in" probe will fire mid-request and return
    // a false positive even when the password is wrong.
    //
    // Truth source: authStore.{isLoggedIn, isAuthPending, tenant, user}.
    // - isLoggedIn=true && isAuthPending=false → confirmed logged in
    // - isAuthPending=true → request in flight, keep polling
    // - isLoggedIn=false && isAuthPending=false → terminal failure;
    //   read inline error from LoginPage if remounted
    let probe_script = r#"(() => {
        const auth = window.__aijia?.authStore?.getState?.();
        if (!auth) return {ready: false, reason: 'authStore_missing'};
        if (auth.isAuthPending) {
            return {ready: false, phase: 'auth_pending'};
        }
        if (auth.isLoggedIn) {
            return {
                ready: true,
                outcome: 'logged_in',
                tenantId: auth.tenant?.id ?? null,
                userId: auth.user?.id ?? null,
            };
        }
        // Terminal not-logged-in: check LoginPage inline error.
        const acct = document.querySelector('#account');
        let inlineError = null;
        if (acct) {
            const err = [...document.querySelectorAll('.text-destructive')]
                .map(el => (el.textContent || '').trim())
                .find(t => t.length > 0);
            if (err) inlineError = err;
        }
        if (inlineError) {
            return {ready: true, outcome: 'login_failed', error: inlineError};
        }
        // Not pending, not logged in, no visible error yet — still
        // transitional (e.g. between request reject and setError flush).
        return {ready: false, phase: 'transitional'};
    })()"#;

    let deadline = Instant::now() + Duration::from_secs(timeout);
    let mut last = Value::Null;
    while Instant::now() < deadline {
        let probe = eval_json(client, probe_script, window).await?;
        last = probe.clone();
        if probe.get("ready").and_then(Value::as_bool) == Some(true) {
            let outcome = probe.get("outcome").and_then(Value::as_str).unwrap_or("");
            return Ok(match outcome {
                "logged_in" => {
                    let tenant_id = probe.get("tenantId").cloned().unwrap_or(Value::Null);
                    let user_id = probe.get("userId").cloned().unwrap_or(Value::Null);
                    let scope = match (&tenant_id, &user_id) {
                        (Value::Number(t), Value::Number(u)) => {
                            Value::String(format!("t_{t}__u_{u}"))
                        }
                        _ => Value::Null,
                    };
                    json!({
                        "ok": true,
                        "outcome": "logged_in",
                        "tenantId": tenant_id,
                        "userId": user_id,
                        "scope": scope,
                    })
                }
                "login_failed" => json!({
                    "ok": false,
                    "reason": "login_failed",
                    "error": probe.get("error").cloned().unwrap_or(Value::Null),
                }),
                _ => json!({"ok": false, "reason": "unknown_ready_outcome", "probe": probe}),
            });
        }
        tokio::time::sleep(Duration::from_millis(250)).await;
    }
    Ok(json!({
        "ok": false,
        "reason": "timeout",
        "timeoutSec": timeout,
        "lastProbe": last,
    }))
}

// ─── compose path: new-task / type-message / send / cancel ───────────────────

async fn new_task(client: &mut Client, wait_fresh: bool, window: Option<&str>) -> Result<Value> {
    // The sidebar's "新任务" entry is rendered as a <button> with that text.
    // Match exclusively on text to dodge layout churn.
    let script = r"(() => {
        const btn = [...document.querySelectorAll('button')]
            .find(b => (b.textContent || '').trim() === '新任务');
        if (!btn) return {ok: false, reason: 'new-task button not found'};
        btn.click();
        return {ok: true};
    })()";
    let click_result = eval_json(client, script, window).await?;
    if click_result.get("ok").and_then(Value::as_bool) != Some(true) {
        return Ok(click_result);
    }

    if !wait_fresh {
        return Ok(click_result);
    }

    // new-task drops back to the "new task" landing page; the store sets
    // activeConversationId = null but does NOT clear `messages` (it's a
    // bag indexed by activeId, components project per-conv slices). The
    // most reliable "ready for fresh input" signal is activeId == null
    // AND the editor exists.
    let probe_script = r"(() => {
        const cs = window.__aijia?.chatStore?.getState?.();
        if (!cs) return {ready: false, reason: 'store missing'};
        const hasEditor = !!document.querySelector('.ProseMirror');
        return {
            ready: cs.activeConversationId == null && hasEditor,
            activeId: cs.activeConversationId,
            hasEditor,
        };
    })()";
    let deadline = Instant::now() + Duration::from_millis(2000);
    let mut last = Value::Null;
    while Instant::now() < deadline {
        let probe = eval_json(client, probe_script, window).await?;
        last = probe.clone();
        if probe.get("ready").and_then(Value::as_bool) == Some(true) {
            return Ok(json!({"ok": true, "fresh": true, "probe": probe}));
        }
        tokio::time::sleep(Duration::from_millis(150)).await;
    }
    Ok(json!({"ok": true, "fresh": false, "reason": "new-task page did not stabilise within 2s", "lastProbe": last}))
}

async fn type_message(client: &mut Client, text: &str, window: Option<&str>) -> Result<Value> {
    // Tiptap (.ProseMirror) ignores synthetic input events. execCommand is the
    // only reliable insertion path the PoC found. The editor must be focused
    // first; we also normalise empty/placeholder state.
    let literal = js_string_literal(text);
    let script = format!(
        r"(() => {{
            const ed = document.querySelector('.ProseMirror');
            if (!ed) return {{ok: false, reason: 'ProseMirror editor not found'}};
            ed.focus();
            const ok = document.execCommand('insertText', false, {literal});
            return {{ok, text: ed.textContent || ''}};
        }})()"
    );
    eval_json(client, &script, window).await
}

async fn send(client: &mut Client, window: Option<&str>) -> Result<Value> {
    let script = r#"(() => {
        const btn = document.querySelector('button[aria-label="发送"]');
        if (!btn) return {ok: false, reason: 'send button not found'};
        if (btn.disabled) return {ok: false, reason: 'send button disabled'};
        btn.click();
        return {ok: true};
    })()"#;
    eval_json(client, script, window).await
}

async fn cancel(client: &mut Client, wait: bool, window: Option<&str>) -> Result<Value> {
    let script = r#"(() => {
        const btn = document.querySelector('button[aria-label="停止"]');
        if (!btn) return {ok: false, reason: 'stop button not found (not streaming?)'};
        btn.click();
        return {ok: true};
    })()"#;
    let click_result = eval_json(client, script, window).await?;
    if click_result.get("ok").and_then(Value::as_bool) != Some(true) {
        return Ok(click_result);
    }

    if !wait {
        return Ok(click_result);
    }

    // Poll until chatStore.isStreaming flips false and the stop button is
    // gone. 5s deadline matches the original cargo-test budget for the same
    // assertion ("run_chat_request 5s 内返回不挂起").
    let probe_script = r##"(() => {
        const cs = window.__aijia?.chatStore?.getState?.();
        if (!cs) return {ready: false, reason: 'store missing'};
        const activeId = cs.activeConversationId;
        const stream = activeId ? cs.streamStates?.[activeId] : null;
        const streaming = !!cs.isStreaming || !!stream?.isStreaming;
        const hasStopBtn = !!document.querySelector('button[aria-label="停止"]');
        return {ready: !streaming && !hasStopBtn, streaming, hasStopBtn};
    })()"##;
    let deadline = Instant::now() + Duration::from_millis(5000);
    let mut last = Value::Null;
    while Instant::now() < deadline {
        let probe = eval_json(client, probe_script, window).await?;
        last = probe.clone();
        if probe.get("ready").and_then(Value::as_bool) == Some(true) {
            return Ok(json!({"ok": true, "cancelled": true, "probe": probe}));
        }
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
    Ok(json!({"ok": false, "reason": "isStreaming did not clear within 5s", "lastProbe": last}))
}

// ─── wait-reply ──────────────────────────────────────────────────────────────

async fn wait_reply(client: &mut Client, timeout: u64, window: Option<&str>) -> Result<Value> {
    let deadline = Instant::now() + Duration::from_secs(timeout);
    let probe_script = r##"(() => {
        const cs = window.__aijia?.chatStore?.getState?.();
        if (!cs) return {ready: false, reason: 'store missing'};
        const activeId = cs.activeConversationId;
        const stream = activeId ? cs.streamStates?.[activeId] : null;
        const streaming = !!cs.isStreaming || !!stream?.isStreaming;
        const hasStopBtn = !!document.querySelector('button[aria-label="停止"]');
        return {ready: !streaming && !hasStopBtn, streaming, hasStopBtn, lastError: stream?.lastError ?? null};
    })()"##;
    // A turn can briefly show isStreaming=false between tool calls — the model
    // pauses while a tool runs, then resumes. Require the "ready" signal to
    // hold for STABILITY_TICKS consecutive probes before declaring done.
    const STABILITY_TICKS: u8 = 3;
    const TICK_MS: u64 = 300;
    let mut consecutive_ready: u8 = 0;
    let mut last = Value::Null;
    while Instant::now() < deadline {
        let probe = eval_json(client, probe_script, window).await?;
        last = probe.clone();
        if probe.get("ready").and_then(Value::as_bool) == Some(true) {
            consecutive_ready = consecutive_ready.saturating_add(1);
            if consecutive_ready >= STABILITY_TICKS {
                // Give the chat store one more beat to flush the final
                // assistant message before returning.
                tokio::time::sleep(Duration::from_millis(200)).await;
                return Ok(json!({"ok": true, "probe": probe, "stableTicks": STABILITY_TICKS}));
            }
        } else {
            consecutive_ready = 0;
        }
        tokio::time::sleep(Duration::from_millis(TICK_MS)).await;
    }
    Ok(json!({"ok": false, "reason": "timeout", "timeoutSec": timeout, "lastProbe": last}))
}

// ─── message readers ─────────────────────────────────────────────────────────

async fn ui_message(
    client: &mut Client,
    last: Option<usize>,
    role: Option<&str>,
    _since: Option<&str>,
    include_tools: bool,
    include_empty: bool,
    window: Option<&str>,
) -> Result<Value> {
    // The store is the source of truth; the DOM is the rendering. Reading the
    // store gives us role, text, and tool_calls in one place. `--since` is not
    // implemented yet (Phase 2A scope); we record the filter in the response so
    // callers can detect drift between request and behaviour.
    let script = r"(() => {
        const cs = window.__aijia?.chatStore?.getState?.();
        if (!cs) return {error: 'store missing'};
        const messages = cs.messages || [];
        return messages.map((m, i) => {
            const text = (m.content && (m.content.text || '')) || '';
            const tool_calls = m.tool_calls || [];
            return {
                index: i,
                role: m.role,
                text,
                tool_calls,
                id: m.id ?? null,
            };
        });
    })()";
    let mut data = eval_json(client, script, window).await?;
    let mut items = match data.take() {
        Value::Array(arr) => arr,
        Value::Object(o) if o.contains_key("error") => {
            return Ok(json!({"ok": false, "reason": o.get("error").cloned().unwrap_or(Value::Null)}));
        }
        _ => Vec::new(),
    };

    // Drop empty placeholder bubbles unless caller opts in. These show up as
    // residue after a cancelled streaming turn — text is "" and no tool_calls
    // were captured — and carry no assertion value.
    if !include_empty {
        items.retain(|m| {
            let has_text = m
                .get("text")
                .and_then(Value::as_str)
                .is_some_and(|s| !s.is_empty());
            let has_tool_calls = m
                .get("tool_calls")
                .and_then(Value::as_array)
                .is_some_and(|a| !a.is_empty());
            has_text || has_tool_calls
        });
    }

    // Filter by role first so --last counts only matching items.
    if let Some(r) = role {
        items.retain(|m| {
            m.get("role")
                .and_then(Value::as_str)
                .map_or(false, |actual| actual == r)
        });
    }

    if !include_tools {
        // Drop tool_calls from the output rather than filtering rows — tool
        // calls live as a field on assistant messages, not separate rows.
        for m in &mut items {
            if let Some(obj) = m.as_object_mut() {
                obj.remove("tool_calls");
            }
        }
    }

    if let Some(n) = last
        && items.len() > n
    {
        items.drain(0..items.len() - n);
    }

    Ok(Value::Array(items))
}

async fn last_reply(client: &mut Client, window: Option<&str>) -> Result<Value> {
    // Equivalent to `ui-message --last 1 --role assistant` but returns a single
    // object (or null) rather than a one-element array — easier to pipe. The
    // empty-placeholder filter is on by default here too, so the last "real"
    // assistant message wins even if a cancelled-turn residue follows it.
    let arr = ui_message(client, Some(1), Some("assistant"), None, true, false, window).await?;
    let first = arr.as_array().and_then(|a| a.first()).cloned();
    Ok(first.unwrap_or(Value::Null))
}

// ─── session management ─────────────────────────────────────────────────────

async fn list_sessions(client: &mut Client, window: Option<&str>) -> Result<Value> {
    let script = r"(() => {
        const cs = window.__aijia?.chatStore?.getState?.();
        if (!cs) return {error: 'store missing'};
        const items = (cs.conversations || []).map((c, i) => ({
            index: i,
            id: c.id,
            title: c.title ?? '',
            active: c.id === cs.activeConversationId,
            // Backend persists `isArchived`; lifecycle stays a fallback for
            // future state-machine work where the field may shift.
            archived: c.isArchived === true || c.archived === true || c.lifecycle === 'Archived',
        }));
        return items;
    })()";
    let data = eval_json(client, script, window).await?;
    Ok(data)
}

async fn switch_session(
    client: &mut Client,
    id_or_index: &str,
    window: Option<&str>,
) -> Result<Value> {
    // Resolve id_or_index against the store first (cheaper + clearer than
    // hunting through DOM rows), then click the matching sidebar row by its
    // data-aijia-conversation-id attribute. Clicking instead of poking the
    // store keeps the navigation path identical to a real user.
    let resolve = format!(
        r##"(() => {{
            const cs = window.__aijia?.chatStore?.getState?.();
            if (!cs) return {{ok:false, reason:'store missing'}};
            const arg = {arg};
            const isIndex = /^\d+$/.test(arg);
            const list = cs.conversations || [];
            const conv = isIndex ? list[parseInt(arg, 10)] : list.find(c => c.id === arg);
            if (!conv) return {{ok:false, reason:'conversation not found', arg}};
            const row = document.querySelector('[data-aijia-conversation-id="' + conv.id + '"]');
            if (!row) return {{ok:false, reason:'row not rendered', id: conv.id}};
            row.click();
            return {{ok:true, id: conv.id, title: conv.title ?? null}};
        }})()"##,
        arg = js_string_literal(id_or_index)
    );
    eval_json(client, &resolve, window).await
}

async fn archive_session(
    client: &mut Client,
    id_or_index: &str,
    wait: bool,
    window: Option<&str>,
) -> Result<Value> {
    // Archiving the entirely-UI way requires hover → menu → confirm dialog.
    // Hover doesn't fire reliably under tauri-pilot's pointer model, so we
    // resolve the conversation id from the store and dispatch the
    // `archive_conversation` Tauri command directly. The end state matches
    // what the UI flow would produce, but skips the menu interaction. When
    // we need to test the UI affordance itself, a separate command can be
    // added; this one is for cleanup / setup.
    let resolve_script = format!(
        r"(() => {{
            const cs = window.__aijia?.chatStore?.getState?.();
            if (!cs) return null;
            const arg = {};
            const isIndex = /^\d+$/.test(arg);
            const list = cs.conversations || [];
            const conv = isIndex ? list[parseInt(arg, 10)] : list.find(c => c.id === arg);
            return conv?.id ?? null;
        }})()",
        js_string_literal(id_or_index)
    );
    let id_val = eval_json(client, &resolve_script, window).await?;
    let id = id_val
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("conversation not found: {id_or_index}"))?
        .to_owned();

    // Tauri 2 IPC converts Rust snake_case args to JS camelCase, so the
    // wire-level key the bridge expects is `conversationId`. Passing
    // `conversation_id` is rejected with "missing required key conversationId".
    let ipc_args = json!({"conversationId": id});
    client
        .call(
            "ipc",
            with_window(
                Some(json!({"command": "archive_conversation", "args": ipc_args})),
                window,
            ),
        )
        .await?;

    if !wait {
        return Ok(json!({"ok": true, "archived": id}));
    }

    // IPC `archive_conversation` writes to disk synchronously but does NOT
    // push a notification back into the frontend chatStore — the UI path
    // (`useChat.archiveConversation`) optimistically filters the row out
    // *before* the IPC call. Since we bypass that path, the store still
    // shows the conversation as active.
    //
    // To get a deterministic confirmation, we poll the *backend* directly
    // via `get_conversations` IPC and look at the authoritative isArchived
    // flag. This is the same query `useChat.loadConversations` makes.
    let confirm_args = json!({});
    let deadline = Instant::now() + Duration::from_millis(3000);
    let mut last = Value::Null;
    while Instant::now() < deadline {
        let convs = client
            .call(
                "ipc",
                with_window(
                    Some(json!({"command": "get_conversations", "args": confirm_args})),
                    window,
                ),
            )
            .await?;
        last = convs.clone();
        if let Some(arr) = convs.as_array() {
            let entry = arr
                .iter()
                .find(|c| c.get("id").and_then(Value::as_str) == Some(id.as_str()));
            let archived = match entry {
                None => true, // hidden from list — treat as archived
                Some(c) => c
                    .get("isArchived")
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
            };
            if archived {
                return Ok(
                    json!({"ok": true, "archived": id, "confirmed": true, "hiddenFromList": entry.is_none()}),
                );
            }
        }
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
    Ok(json!({"ok": false, "archived": id, "reason": "backend get_conversations did not show isArchived=true within 3s", "lastResponse": last}))
}

async fn select_workspace(client: &mut Client, name: &str, window: Option<&str>) -> Result<Value> {
    let _ = (client, window);
    Ok(json!({
        "ok": false,
        "reason": "select-workspace not implemented yet (workspace picker selectors not finalised)",
        "name": name,
    }))
}

async fn restart_app(client: &mut Client, window: Option<&str>) -> Result<Value> {
    let _ = (client, window);
    Ok(json!({
        "ok": false,
        "reason": "restart-app not implemented yet (would sever the pilot socket; needs reconnect protocol)",
    }))
}

async fn screenshot(
    client: &mut Client,
    label: &str,
    selector: Option<&str>,
    window: Option<&str>,
) -> Result<Value> {
    // Thin wrapper around the underlying `screenshot` RPC. All heavy lifting —
    // capture, fallback, timeout handling — belongs in the plugin layer, not
    // here. We just stamp the file name and forward the selector. If callers
    // hit a timeout, the fix lives in tauri-plugin-pilot, not in aijia.rs.
    let safe: String = label
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '-'
            }
        })
        .collect();
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let path = format!("/tmp/aijia-e2e-{safe}-{ts}.png");

    let effective_selector = if let Some(s) = selector {
        Some(s.to_owned())
    } else {
        // Default to the message-list anchor: even with a fast capture path
        // this gives a tighter, more readable region than full-document.
        let probe = eval_json(
            client,
            "(() => document.querySelector('[data-aijia-message-list]') ? '[data-aijia-message-list]' : null)()",
            window,
        )
        .await
        .unwrap_or(Value::Null);
        probe.as_str().map(str::to_owned)
    };

    let mut rpc_params = json!({"path": null});
    if let Some(ref s) = effective_selector {
        rpc_params["selector"] = json!(s);
    }
    let result = client
        .call("screenshot", with_window(Some(rpc_params), window))
        .await?;
    let data_url = result
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("screenshot result not a string"))?;
    let base64_data = data_url
        .strip_prefix("data:image/png;base64,")
        .unwrap_or(data_url);
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(base64_data)
        .map_err(|e| anyhow::anyhow!("failed to decode base64: {e}"))?;
    std::fs::write(&path, bytes).with_context(|| format!("write screenshot to {path}"))?;
    Ok(json!({"ok": true, "path": path, "selector": effective_selector}))
}

async fn cleanup_test_sessions(
    client: &mut Client,
    prefix: &str,
    window: Option<&str>,
) -> Result<Value> {
    // Match by conversation title prefix. Conversation messages live in the
    // chat store keyed by *active* conversation id only, so we can't peek at
    // first-user-message contents for non-active sessions — title is the only
    // bulk-accessible field.
    //
    // To make a session match, test authors must rename the conversation
    // (UI: ⋯ → 重命名聊天) so the title starts with the prefix. Just sending
    // a message that begins with the prefix does NOT change the title.
    let collect_script = format!(
        r##"(() => {{
            const cs = window.__aijia?.chatStore?.getState?.();
            if (!cs) return [];
            const list = cs.conversations || [];
            const prefix = {prefix_lit};
            const out = [];
            for (const c of list) {{
                if (c.isArchived === true || c.archived === true || c.lifecycle === 'Archived') continue;
                if (typeof c.title === 'string' && c.title.startsWith(prefix)) {{
                    out.push({{id: c.id, title: c.title}});
                }}
            }}
            return out;
        }})()"##,
        prefix_lit = js_string_literal(prefix)
    );
    let candidates = eval_json(client, &collect_script, window).await?;
    let list = candidates.as_array().cloned().unwrap_or_default();
    let mut archived = Vec::new();
    let mut failed: Vec<Value> = Vec::new();
    for entry in list {
        let id = entry.get("id").and_then(Value::as_str).unwrap_or("");
        if id.is_empty() {
            continue;
        }
        let res = client
            .call(
                "ipc",
                with_window(
                    Some(json!({
                        "command": "archive_conversation",
                        "args": {"conversationId": id},
                    })),
                    window,
                ),
            )
            .await;
        match res {
            Ok(_) => archived.push(entry),
            Err(e) => failed.push(json!({"id": id, "error": e.to_string()})),
        }
    }
    Ok(json!({"ok": failed.is_empty(), "prefix": prefix, "archived": archived, "failed": failed}))
}

use base64::Engine as _;

// ─── composer: attachment picker (queue + click +) ───────────────────────────

async fn composer_queue_files(
    client: &mut Client,
    paths_csv: &str,
    window: Option<&str>,
) -> Result<Value> {
    let paths: Vec<&str> = paths_csv
        .split(',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect();
    if paths.is_empty() {
        return Ok(json!({
            "ok": false,
            "reason": "no_paths",
            "hint": "pass --paths as a comma-separated list of absolute paths",
        }));
    }
    let paths_json = serde_json::to_string(&paths)?;
    let script = format!(
        r#"(() => {{
            const aj = window.__aijia;
            if (!aj) return {{ok: false, reason: 'dev_hooks_unavailable'}};
            if (!Array.isArray(aj._pickAttachmentsMockQueue)) {{
                aj._pickAttachmentsMockQueue = [];
            }}
            const paths = {paths};
            aj._pickAttachmentsMockQueue.push(paths);
            return {{
                ok: true,
                queued: paths.length,
                queueDepth: aj._pickAttachmentsMockQueue.length,
            }};
        }})()"#,
        paths = paths_json,
    );
    eval_json(client, &script, window).await
}

async fn composer_click_plus(client: &mut Client, window: Option<&str>) -> Result<Value> {
    let script = r#"(() => {
        const btn = document.querySelector('[data-aijia-composer-plus]');
        if (!btn) return {ok: false, reason: 'composer_plus_button_not_found'};
        if (btn.disabled) return {ok: false, reason: 'composer_plus_button_disabled'};
        btn.click();
        return {ok: true};
    })()"#;
    eval_json(client, script, window).await
}

// ─── settings + logout: atomic ──────────────────────────────────────────────

async fn open_settings(client: &mut Client, window: Option<&str>) -> Result<Value> {
    let script = r#"(() => {
        const btn = document.querySelector('[data-aijia-open-settings]');
        if (!btn) return {ok: false, reason: 'open_settings_button_not_found'};
        btn.click();
        return {ok: true};
    })()"#;
    eval_json(client, script, window).await
}

async fn settings_wait(
    client: &mut Client,
    timeout_sec: u64,
    window: Option<&str>,
) -> Result<Value> {
    let probe = r"(() => ({ready: !!document.querySelector('[data-aijia-settings-shell]')}))()";
    let last = wait_until(client, probe, timeout_sec * 1000, window).await?;
    if last.get("ready").and_then(Value::as_bool) == Some(true) {
        return Ok(json!({"ok": true}));
    }
    Ok(json!({
        "ok": false,
        "reason": "timeout",
        "timeoutSec": timeout_sec,
    }))
}

async fn settings_select_panel(
    client: &mut Client,
    key: &str,
    window: Option<&str>,
) -> Result<Value> {
    let k_lit = js_string_literal(key);
    let script = format!(
        r#"(() => {{
            const shell = document.querySelector('[data-aijia-settings-shell]');
            if (!shell) return {{ok: false, reason: 'settings_not_open'}};
            const btn = shell.querySelector('[data-aijia-settings-panel="' + {k} + '"]');
            if (!btn) return {{
                ok: false,
                reason: 'panel_not_found',
                requested: {k},
                available: [...shell.querySelectorAll('[data-aijia-settings-panel]')]
                    .map(b => b.getAttribute('data-aijia-settings-panel')),
            }};
            btn.click();
            return {{ok: true, key: {k}}};
        }})()"#,
        k = k_lit,
    );
    eval_json(client, &script, window).await
}

async fn settings_close(client: &mut Client, window: Option<&str>) -> Result<Value> {
    let script = r#"(() => {
        const shell = document.querySelector('[data-aijia-settings-shell]');
        if (!shell) return {ok: false, reason: 'settings_not_open'};
        const btn = shell.querySelector('[data-aijia-settings-action="close"]');
        if (!btn) return {ok: false, reason: 'close_button_not_found'};
        btn.click();
        return {ok: true};
    })()"#;
    eval_json(client, script, window).await
}

async fn logout(client: &mut Client, window: Option<&str>) -> Result<Value> {
    // Pre: settings modal open + General panel active. The button has no
    // confirm dialog — auth state flips synchronously, settings modal closes
    // itself via closeSettings(), and AuthGate re-mounts LoginPage. Caller
    // can chain `where --json` polling on loggedIn===false to confirm.
    let script = r#"(() => {
        const btn = document.querySelector('[data-aijia-logout-button]');
        if (!btn) return {ok: false, reason: 'logout_button_not_found_or_panel_inactive'};
        if (btn.disabled) return {ok: false, reason: 'logout_button_disabled'};
        btn.click();
        return {ok: true};
    })()"#;
    eval_json(client, script, window).await
}

// ─── agenda: atomic editor ops ──────────────────────────────────────────────

async fn agenda_open_new(client: &mut Client, window: Option<&str>) -> Result<Value> {
    let script = r#"(() => {
        const btn = document.querySelector('[data-aijia-agenda-new]');
        if (!btn) return {ok: false, reason: 'agenda_new_button_not_found'};
        if (btn.disabled) return {ok: false, reason: 'agenda_new_button_disabled'};
        btn.click();
        return {ok: true};
    })()"#;
    eval_json(client, script, window).await
}

async fn agenda_wait_editor(
    client: &mut Client,
    timeout_sec: u64,
    want_open: bool,
    window: Option<&str>,
) -> Result<Value> {
    let probe = if want_open {
        r"(() => ({ready: !!document.querySelector('[data-aijia-agenda-editor]')}))()"
    } else {
        r"(() => ({ready: !document.querySelector('[data-aijia-agenda-editor]')}))()"
    };
    let last = wait_until(client, probe, timeout_sec * 1000, window).await?;
    if last.get("ready").and_then(Value::as_bool) == Some(true) {
        return Ok(json!({"ok": true, "wantOpen": want_open}));
    }
    Ok(json!({
        "ok": false,
        "reason": "timeout",
        "wantOpen": want_open,
        "timeoutSec": timeout_sec,
    }))
}

async fn agenda_fill(
    client: &mut Client,
    field: &str,
    value: &str,
    window: Option<&str>,
) -> Result<Value> {
    if !["title", "prompt"].contains(&field) {
        return Ok(json!({
            "ok": false,
            "reason": "invalid_field",
            "expected": ["title", "prompt"],
            "got": field,
        }));
    }
    let field_lit = js_string_literal(field);
    let value_lit = js_string_literal(value);
    let script = format!(
        r#"(() => {{
            const root = document.querySelector('[data-aijia-agenda-editor]');
            if (!root) return {{ok: false, reason: 'editor_not_open'}};
            const el = root.querySelector('[data-aijia-agenda-field="' + {f} + '"]');
            if (!el) return {{ok: false, reason: 'field_missing', field: {f}}};
            const proto = el.tagName === 'TEXTAREA'
                ? window.HTMLTextAreaElement.prototype
                : window.HTMLInputElement.prototype;
            const setter = Object.getOwnPropertyDescriptor(proto, 'value').set;
            setter.call(el, {v});
            el.dispatchEvent(new Event('input', {{bubbles: true}}));
            return {{ok: true, field: {f}, value: el.value}};
        }})()"#,
        f = field_lit,
        v = value_lit,
    );
    eval_json(client, &script, window).await
}

/// Map CLI-friendly frequency aliases (`once`, `weekly`, ...) to the editor's
/// internal value (`one_shot`, `weekly`, ...). The editor's `<select>` uses
/// the on-disk `Freq` enum plus a `one_shot` sentinel for null rule.
fn map_frequency(cli: &str) -> Option<&'static str> {
    match cli {
        "once" | "one_shot" | "one-shot" => Some("one_shot"),
        "daily" => Some("daily"),
        "weekly" => Some("weekly"),
        "monthly" => Some("monthly"),
        "yearly" => Some("yearly"),
        _ => None,
    }
}

async fn agenda_set_frequency(
    client: &mut Client,
    value: &str,
    window: Option<&str>,
) -> Result<Value> {
    let mapped = match map_frequency(value) {
        Some(v) => v,
        None => {
            return Ok(json!({
                "ok": false,
                "reason": "invalid_frequency",
                "expected": ["once", "daily", "weekly", "monthly", "yearly"],
                "got": value,
            }));
        }
    };
    let v_lit = js_string_literal(mapped);
    let script = format!(
        r#"(() => {{
            const root = document.querySelector('[data-aijia-agenda-editor]');
            if (!root) return {{ok: false, reason: 'editor_not_open'}};
            const sel = root.querySelector('select[aria-label="频率"]');
            if (!sel) return {{ok: false, reason: 'frequency_select_missing'}};
            const want = {v};
            const opt = [...sel.options].find(o => o.value === want);
            if (!opt) return {{ok: false, reason: 'frequency_option_missing', got: want}};
            const setter = Object.getOwnPropertyDescriptor(window.HTMLSelectElement.prototype, 'value').set;
            setter.call(sel, opt.value);
            sel.dispatchEvent(new Event('change', {{bubbles: true}}));
            return {{ok: true, value: sel.value}};
        }})()"#,
        v = v_lit,
    );
    eval_json(client, &script, window).await
}

async fn agenda_set_start_at(
    client: &mut Client,
    value: &str,
    window: Option<&str>,
) -> Result<Value> {
    let v_lit = js_string_literal(value);
    let script = format!(
        r#"(() => {{
            const root = document.querySelector('[data-aijia-agenda-editor]');
            if (!root) return {{ok: false, reason: 'editor_not_open'}};
            const el = root.querySelector('#agenda-editor-start');
            if (!el) return {{ok: false, reason: 'start_input_missing'}};
            const setter = Object.getOwnPropertyDescriptor(window.HTMLInputElement.prototype, 'value').set;
            setter.call(el, {v});
            el.dispatchEvent(new Event('input', {{bubbles: true}}));
            return {{ok: true, value: el.value}};
        }})()"#,
        v = v_lit,
    );
    eval_json(client, &script, window).await
}

async fn agenda_set_employee(
    client: &mut Client,
    name: &str,
    window: Option<&str>,
) -> Result<Value> {
    let n_lit = js_string_literal(name);
    let script = format!(
        r#"(() => {{
            const root = document.querySelector('[data-aijia-agenda-editor]');
            if (!root) return {{ok: false, reason: 'editor_not_open'}};
            const sel = root.querySelector('select[aria-label="执行员工"]');
            if (!sel) return {{ok: false, reason: 'employee_select_missing_or_no_employees'}};
            if (sel.disabled) return {{ok: false, reason: 'employee_select_disabled', hint: 'editing_existing_agenda'}};
            const want = {n};
            const opt = [...sel.options].find(o => (o.textContent || '').includes(want));
            if (!opt) return {{
                ok: false,
                reason: 'employee_not_found',
                requested: want,
                options: [...sel.options].map(o => o.textContent),
            }};
            const setter = Object.getOwnPropertyDescriptor(window.HTMLSelectElement.prototype, 'value').set;
            setter.call(sel, opt.value);
            sel.dispatchEvent(new Event('change', {{bubbles: true}}));
            return {{ok: true, employeeId: opt.value, optionText: opt.textContent}};
        }})()"#,
        n = n_lit,
    );
    eval_json(client, &script, window).await
}

/// Click "保存" or "取消" inside the agenda editor. Atomic — does NOT wait
/// for the editor to close (chain `agenda-wait-editor-closed`).
async fn agenda_editor_action(
    client: &mut Client,
    action: &str,
    window: Option<&str>,
) -> Result<Value> {
    let action_lit = js_string_literal(action);
    let script = format!(
        r#"(() => {{
            const root = document.querySelector('[data-aijia-agenda-editor]');
            if (!root) return {{ok: false, reason: 'editor_not_open'}};
            const btn = root.querySelector('[data-aijia-agenda-action="' + {a} + '"]');
            if (!btn) return {{ok: false, reason: 'action_button_missing', action: {a}}};
            if (btn.disabled) return {{ok: false, reason: 'action_button_disabled', action: {a}}};
            btn.click();
            return {{ok: true, action: {a}}};
        }})()"#,
        a = action_lit,
    );
    eval_json(client, &script, window).await
}

async fn agenda_wait_row(
    client: &mut Client,
    title: &str,
    timeout_sec: u64,
    window: Option<&str>,
) -> Result<Value> {
    let t_lit = js_string_literal(title);
    let probe = format!(
        r#"(() => {{
            const want = {t};
            const row = [...document.querySelectorAll('[data-aijia-agenda-row]')]
                .find(r => r.getAttribute('data-aijia-agenda-title') === want);
            if (!row) return {{ready: false}};
            return {{
                ready: true,
                agendaId: row.getAttribute('data-aijia-agenda-id'),
                status: row.getAttribute('data-aijia-agenda-status'),
            }};
        }})()"#,
        t = t_lit,
    );
    let last = wait_until(client, &probe, timeout_sec * 1000, window).await?;
    if last.get("ready").and_then(Value::as_bool) == Some(true) {
        return Ok(json!({
            "ok": true,
            "title": title,
            "agendaId": last.get("agendaId").cloned().unwrap_or(Value::Null),
            "status": last.get("status").cloned().unwrap_or(Value::Null),
        }));
    }
    Ok(json!({
        "ok": false,
        "reason": "timeout",
        "timeoutSec": timeout_sec,
    }))
}

/// Click a single action button on the row matching `title`. Each row has
/// hover-revealed buttons with `aria-label="<verb> <title>"`. `cancel` opens
/// a ConfirmDialog — caller chains `handle-dialog --action accept`; this
/// command does NOT auto-confirm.
async fn agenda_row_action(
    client: &mut Client,
    title: &str,
    action: &str,
    window: Option<&str>,
) -> Result<Value> {
    let aria_prefix = match action {
        "run-now" | "run_now" => "立即运行 ",
        "pause" => "暂停 ",
        "resume" => "启用 ",
        "edit" => "编辑 ",
        "cancel" => "取消 ",
        "restore" => "恢复 ",
        "purge" => "永久删除 ",
        _ => {
            return Ok(json!({
                "ok": false,
                "reason": "invalid_action",
                "expected": ["run-now", "pause", "resume", "edit", "cancel", "restore", "purge"],
                "got": action,
            }));
        }
    };
    let t_lit = js_string_literal(title);
    let aria_lit = js_string_literal(aria_prefix);
    let script = format!(
        r#"(() => {{
            const want = {t};
            const row = [...document.querySelectorAll('[data-aijia-agenda-row]')]
                .find(r => r.getAttribute('data-aijia-agenda-title') === want);
            if (!row) return {{ok: false, reason: 'row_not_found', title: want}};
            const btn = [...row.querySelectorAll('button')]
                .find(b => (b.getAttribute('aria-label') || '').startsWith({a}));
            if (!btn) return {{ok: false, reason: 'action_button_not_found', action: {a}}};
            if (btn.disabled) return {{ok: false, reason: 'action_button_disabled', action: {a}}};
            btn.click();
            return {{ok: true, action: {a}, agendaId: row.getAttribute('data-aijia-agenda-id')}};
        }})()"#,
        t = t_lit,
        a = aria_lit,
    );
    eval_json(client, &script, window).await
}

// ─── employee: atomic drawer ops ────────────────────────────────────────────

async fn employee_open_card(
    client: &mut Client,
    name: Option<&str>,
    id: Option<&str>,
    window: Option<&str>,
) -> Result<Value> {
    if name.is_none() && id.is_none() {
        return Ok(json!({
            "ok": false,
            "reason": "missing_argument",
            "hint": "pass exactly one of --name or --id",
        }));
    }
    let (selector_field, want_lit) = match (name, id) {
        (Some(n), None) => ("data-aijia-employee-name", js_string_literal(n)),
        (None, Some(i)) => ("data-aijia-employee-id", js_string_literal(i)),
        _ => unreachable!("clap conflicts_with prevents both"),
    };
    let script = format!(
        r#"(() => {{
            const field = {field};
            const want = {want};
            const card = [...document.querySelectorAll('[data-aijia-employee-card]')]
                .find(c => c.getAttribute(field) === want);
            if (!card) return {{
                ok: false,
                reason: 'employee_card_not_found',
                requested: want,
                searchedField: field,
                cards: [...document.querySelectorAll('[data-aijia-employee-card]')]
                    .map(c => ({{
                        id: c.getAttribute('data-aijia-employee-id'),
                        name: c.getAttribute('data-aijia-employee-name'),
                    }})),
            }};
            card.click();
            return {{
                ok: true,
                employeeId: card.getAttribute('data-aijia-employee-id'),
                name: card.getAttribute('data-aijia-employee-name'),
            }};
        }})()"#,
        field = js_string_literal(selector_field),
        want = want_lit,
    );
    eval_json(client, &script, window).await
}

async fn employee_wait_drawer(
    client: &mut Client,
    timeout_sec: u64,
    window: Option<&str>,
) -> Result<Value> {
    let probe = r"(() => ({ready: !!document.querySelector('[data-aijia-employee-drawer]')}))()";
    let last = wait_until(client, probe, timeout_sec * 1000, window).await?;
    if last.get("ready").and_then(Value::as_bool) == Some(true) {
        return Ok(json!({"ok": true}));
    }
    Ok(json!({
        "ok": false,
        "reason": "timeout",
        "timeoutSec": timeout_sec,
    }))
}

async fn employee_click_dispatch(
    client: &mut Client,
    window: Option<&str>,
) -> Result<Value> {
    let script = r#"(() => {
        const dr = document.querySelector('[data-aijia-employee-drawer]');
        if (!dr) return {ok: false, reason: 'drawer_not_open'};
        const btn = dr.querySelector('[data-aijia-employee-action="dispatch"]');
        if (!btn) return {ok: false, reason: 'dispatch_button_not_found'};
        if (btn.disabled) return {ok: false, reason: 'dispatch_button_disabled', text: (btn.textContent || '').trim()};
        btn.click();
        return {ok: true};
    })()"#;
    eval_json(client, script, window).await
}

async fn employee_close_drawer(
    client: &mut Client,
    window: Option<&str>,
) -> Result<Value> {
    let script = r#"(() => {
        const dr = document.querySelector('[data-aijia-employee-drawer]');
        if (!dr) return {ok: false, reason: 'drawer_not_open'};
        const btn = dr.querySelector('[data-aijia-employee-action="close"]');
        if (!btn) return {ok: false, reason: 'close_button_not_found'};
        btn.click();
        return {ok: true};
    })()"#;
    eval_json(client, script, window).await
}

/// Generic poll-until helper used across agenda flows. probe_script must
/// return `{ready: bool, ...}`; returns the last probe payload.
async fn wait_until(
    client: &mut Client,
    probe_script: &str,
    timeout_ms: u64,
    window: Option<&str>,
) -> Result<Value> {
    let deadline = Instant::now() + Duration::from_millis(timeout_ms);
    let mut last = Value::Null;
    while Instant::now() < deadline {
        let r = eval_json(client, probe_script, window).await?;
        last = r.clone();
        if r.get("ready").and_then(Value::as_bool) == Some(true) {
            return Ok(r);
        }
        tokio::time::sleep(Duration::from_millis(150)).await;
    }
    Ok(last)
}

// ─── navigation: goto ────────────────────────────────────────────────────────

async fn goto(
    client: &mut Client,
    page: &str,
    wait: bool,
    window: Option<&str>,
) -> Result<Value> {
    // Top-level sidebar entries are tagged with `data-aijia-nav={key}`
    // (see `src/components/sidebar/SidebarNav.tsx`). Stable across i18n
    // and layout changes — never match by textContent.
    const VALID: &[&str] = &[
        "home",
        "employees",
        "expert-teams",
        "skill-center",
        "schedules",
        "channel",
    ];
    if !VALID.contains(&page) {
        return Ok(json!({
            "ok": false,
            "reason": "invalid_page",
            "valid": VALID,
            "got": page,
        }));
    }
    let page_lit = js_string_literal(page);
    let click_script = format!(
        r#"(() => {{
            const btn = document.querySelector('[data-aijia-nav="' + {page} + '"]');
            if (!btn) return {{ok: false, reason: 'nav_button_not_found', page: {page}}};
            btn.click();
            return {{ok: true, page: {page}}};
        }})()"#,
        page = page_lit,
    );
    let click = eval_json(client, &click_script, window).await?;
    if click.get("ok").and_then(Value::as_bool) != Some(true) {
        return Ok(click);
    }
    if !wait {
        return Ok(click);
    }

    // Poll uiStore.route.kind until it flips to the requested page.
    // SidebarNav 的 onSelect 是同步 `setRoute({kind})`，正常 < 100ms 完成；
    // 但 Sheet/Dialog 关闭动画可能短暂阻塞 React commit，所以 5s 兜底。
    let probe = format!(
        r#"(() => {{
            const ui = window.__aijia?.uiStore?.getState?.();
            const got = ui?.route?.kind ?? null;
            return {{ready: got === {page}, got: got}};
        }})()"#,
        page = page_lit,
    );
    let last = wait_until(client, &probe, 5000, window).await?;
    if last.get("ready").and_then(Value::as_bool) == Some(true) {
        return Ok(json!({"ok": true, "page": page}));
    }
    Ok(json!({
        "ok": false,
        "reason": "route_did_not_settle",
        "page": page,
        "lastRoute": last.get("got").cloned().unwrap_or(Value::Null),
    }))
}

// ─── dialog: snapshot + click ───────────────────────────────────────────────
//
// 统一查询 / click `[data-aijia-dialog]` 容器，覆盖 permission-ask /
// ask-user-question / confirm 三类弹窗。

async fn dialog_snapshot(client: &mut Client, window: Option<&str>) -> Result<Value> {
    let script = r#"(() => {
        const dialog = document.querySelector('[data-aijia-dialog]');
        if (!dialog) {
            return {ok: true, dialog: null};
        }
        const ds = dialog.dataset || {};
        const title = dialog.querySelector('[data-aijia-dialog-title]')?.textContent?.trim() || null;
        const description = dialog.querySelector('[data-aijia-dialog-description]')?.textContent?.trim() || null;
        const actions = Array.from(dialog.querySelectorAll('[data-aijia-dialog-action]')).map(el => {
            const ed = el.dataset || {};
            const item = {
                action: ed.aijiaDialogAction || null,
                label: el.textContent?.trim() || null,
            };
            if (ed.aijiaDialogQuestionIndex != null && ed.aijiaDialogQuestionIndex !== '') {
                item.questionIndex = Number(ed.aijiaDialogQuestionIndex);
            }
            if (ed.aijiaDialogOptionIndex != null && ed.aijiaDialogOptionIndex !== '') {
                item.optionIndex = Number(ed.aijiaDialogOptionIndex);
            }
            if (ed.aijiaDialogOptionLabel != null && ed.aijiaDialogOptionLabel !== '') {
                item.optionLabel = ed.aijiaDialogOptionLabel;
            }
            return item;
        });
        return {
            ok: true,
            dialog: {
                kind: ds.aijiaDialog || null,
                tool: ds.aijiaDialogTool || null,
                title: title,
                description: description,
                actions: actions,
            },
        };
    })()"#;
    eval_json(client, script, window).await
}

async fn dialog_click(
    client: &mut Client,
    action: &str,
    question_index: Option<u32>,
    option_index: Option<u32>,
    timeout: u64,
    window: Option<&str>,
) -> Result<Value> {
    match action {
        "allow" | "deny" | "cancel" | "confirm" | "option" => {}
        _ => {
            return Ok(json!({
                "ok": false,
                "reason": "invalid_action",
                "expected": ["allow", "deny", "cancel", "confirm", "option"],
                "got": action,
            }));
        }
    }
    if action != "option" && (question_index.is_some() || option_index.is_some()) {
        return Ok(json!({
            "ok": false,
            "reason": "indices_only_valid_for_option",
            "hint": "--question-index / --option-index only apply to --action option",
            "action": action,
        }));
    }
    let action_lit = js_string_literal(action);
    let q_lit = match question_index {
        None => "null".to_string(),
        Some(n) => n.to_string(),
    };
    let o_lit = match option_index {
        None => "null".to_string(),
        Some(n) => n.to_string(),
    };
    let probe_script = format!(
        r#"(() => {{
            const dialog = document.querySelector('[data-aijia-dialog]');
            if (!dialog) return {{ready: false, reason: 'no_dialog'}};
            const action = {action};
            const wantQ = {q};
            const wantO = {o};
            let buttons = Array.from(dialog.querySelectorAll('[data-aijia-dialog-action="' + action + '"]'));
            if (action === 'option') {{
                if (wantQ != null) {{
                    buttons = buttons.filter(b => Number(b.dataset.aijiaDialogQuestionIndex) === wantQ);
                }}
                if (wantO != null) {{
                    buttons = buttons.filter(b => Number(b.dataset.aijiaDialogOptionIndex) === wantO);
                }}
            }}
            return {{ready: buttons.length === 1, hasDialog: true, count: buttons.length}};
        }})()"#,
        action = action_lit,
        q = q_lit,
        o = o_lit,
    );
    let click_script = format!(
        r#"(() => {{
            const dialog = document.querySelector('[data-aijia-dialog]');
            if (!dialog) return {{ok: false, reason: 'no_dialog'}};
            const action = {action};
            const wantQ = {q};
            const wantO = {o};
            let buttons = Array.from(dialog.querySelectorAll('[data-aijia-dialog-action="' + action + '"]'));
            if (action === 'option') {{
                if (wantQ != null) {{
                    buttons = buttons.filter(b => Number(b.dataset.aijiaDialogQuestionIndex) === wantQ);
                }}
                if (wantO != null) {{
                    buttons = buttons.filter(b => Number(b.dataset.aijiaDialogOptionIndex) === wantO);
                }}
            }}
            if (buttons.length === 0) {{
                return {{ok: false, reason: 'action_button_not_found', action: action}};
            }}
            if (buttons.length > 1) {{
                return {{
                    ok: false,
                    reason: 'ambiguous_action_button',
                    action: action,
                    count: buttons.length,
                    hint: 'specify --question-index / --option-index to disambiguate',
                }};
            }}
            const btn = buttons[0];
            if (btn.disabled) {{
                return {{ok: false, reason: 'action_button_disabled', action: action}};
            }}
            btn.click();
            return {{
                ok: true,
                action: action,
                questionIndex: btn.dataset.aijiaDialogQuestionIndex != null
                    && btn.dataset.aijiaDialogQuestionIndex !== ''
                    ? Number(btn.dataset.aijiaDialogQuestionIndex)
                    : null,
                optionIndex: btn.dataset.aijiaDialogOptionIndex != null
                    && btn.dataset.aijiaDialogOptionIndex !== ''
                    ? Number(btn.dataset.aijiaDialogOptionIndex)
                    : null,
            }};
        }})()"#,
        action = action_lit,
        q = q_lit,
        o = o_lit,
    );

    let deadline = Instant::now() + Duration::from_secs(timeout);
    let mut last = Value::Null;
    while Instant::now() < deadline {
        let probe = eval_json(client, &probe_script, window).await?;
        last = probe.clone();
        if probe.get("ready").and_then(Value::as_bool) == Some(true) {
            return eval_json(client, &click_script, window).await;
        }
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
    Ok(json!({
        "ok": false,
        "reason": "timeout",
        "timeoutSec": timeout,
        "lastProbe": last,
    }))
}

// ─── tool-call introspection ────────────────────────────────────────────────

fn parse_turn_arg(turn: &str) -> Result<Option<usize>> {
    if turn == "last" {
        return Ok(None);
    }
    let n: usize = turn
        .parse()
        .with_context(|| format!("--turn must be `last` or a non-negative integer, got `{turn}`"))?;
    Ok(Some(n))
}

async fn tool_calls(client: &mut Client, turn: &str, window: Option<&str>) -> Result<Value> {
    // Walk chatStore.messages, group by turn. A "turn" starts at a user
    // message and ends right before the next user message; intermediate
    // assistant + tool messages belong to it. Returns the tool_calls of
    // the requested turn.
    let n = parse_turn_arg(turn)?;
    let n_lit = match n {
        None => "null".to_string(),
        Some(idx) => idx.to_string(),
    };
    let script = format!(
        r#"(() => {{
            const cs = window.__aijia?.chatStore?.getState?.();
            if (!cs) return {{ok: false, reason: 'store_missing'}};
            const msgs = cs.messages || [];
            const turns = [];
            let cur = null;
            for (const m of msgs) {{
                if (m.role === 'user') {{
                    if (cur) turns.push(cur);
                    cur = {{ role: 'user', userText: (m.content?.text || ''), tools: [] }};
                }} else if (cur) {{
                    const calls = m.tool_calls || [];
                    for (const c of calls) cur.tools.push({{
                        tool_name: c.name ?? c.tool_name ?? null,
                        args: c.args ?? c.input ?? null,
                        status: c.status ?? null,
                        result_summary: typeof c.result === 'string'
                            ? c.result.slice(0, 200)
                            : (c.result ? JSON.stringify(c.result).slice(0, 200) : null),
                    }});
                }}
            }}
            if (cur) turns.push(cur);
            if (turns.length === 0) return {{ok: false, reason: 'no_turns'}};
            const idx = ({n} == null) ? (turns.length - 1) : {n};
            const t = turns[idx];
            if (!t) return {{ok: false, reason: 'turn_out_of_range', turnCount: turns.length, requested: idx}};
            return {{ok: true, turn: idx, turnCount: turns.length, userText: t.userText, tools: t.tools}};
        }})()"#,
        n = n_lit,
    );
    eval_json(client, &script, window).await
}

async fn tool_bubble(client: &mut Client, turn: &str, window: Option<&str>) -> Result<Value> {
    // Scrape currently-rendered tool bubbles from the chat DOM, bypassing
    // ui-message's "drop empty placeholder" filter. Useful when verifying
    // post-restart UI: the jsonl on disk is correct but ui-message hides
    // bubbles whose text == "" because tool_calls metadata is reattached
    // out of band.
    //
    // The current chat layout doesn't have a per-turn DOM marker yet, so
    // `--turn last` is the only meaningful value today. `--turn N` returns
    // the same payload with a note. Once turn boundaries are tagged in DOM
    // (e.g. `data-aijia-turn={n}`) this command can split them properly.
    let n = parse_turn_arg(turn)?;
    let script = r#"(() => {
        // Heuristic: look for our streaming + AI bubbles and their child
        // tool blocks. Falls back to any element with a class containing
        // 'tool-call' or 'tool-bubble'. Best-effort — author should add
        // `data-aijia-tool-bubble` when the bubble component is touched.
        const bubbles = [...document.querySelectorAll(
            '[data-aijia-tool-bubble], [data-aijia-ai-bubble] [class*="tool"], [data-aijia-streaming-bubble] [class*="tool"]'
        )];
        const out = bubbles.map((el) => {
            const text = (el.textContent || '').trim();
            const expanded = el.getAttribute('data-aijia-tool-expanded') === 'true'
                || el.getAttribute('aria-expanded') === 'true';
            const hasSpinner = !!el.querySelector('[class*="animate-spin"], [data-aijia-spinner]');
            const errorEl = el.querySelector('[class*="text-destructive"]');
            const errorText = errorEl ? (errorEl.textContent || '').trim() : null;
            const linkEl = el.querySelector('a[href]');
            const firstUrl = linkEl ? linkEl.getAttribute('href') : null;
            const status = errorText
                ? 'failed'
                : (hasSpinner ? 'running' : 'succeeded');
            // tool name often surfaces as the first short heading or a
            // bolded span; this is heuristic and may miss bespoke layouts.
            const headingEl = el.querySelector('strong, [class*="font-medium"], [class*="font-semibold"]');
            const tool_name = headingEl ? (headingEl.textContent || '').trim() || null : null;
            return {
                tool_name,
                status,
                expanded,
                first_url: firstUrl,
                error_text: errorText,
                preview: text.slice(0, 200),
            };
        });
        return {ok: true, bubbles: out, count: out.length};
    })()"#;
    let mut data = eval_json(client, script, window).await?;
    if let Some(obj) = data.as_object_mut() {
        obj.insert(
            "turn_arg".to_string(),
            match n {
                Some(i) => json!(i),
                None => json!("last"),
            },
        );
        if n.is_some() {
            obj.insert(
                "note".to_string(),
                json!("turn boundaries not yet tagged in DOM — payload covers all currently-rendered bubbles"),
            );
        }
    }
    Ok(data)
}

// ─── hire wizard: atomic ops ────────────────────────────────────────────────

async fn hire_open(client: &mut Client, variant: &str, window: Option<&str>) -> Result<Value> {
    if !["template-market", "add-card"].contains(&variant) {
        return Ok(json!({
            "ok": false,
            "reason": "invalid_variant",
            "expected": ["template-market", "add-card"],
            "got": variant,
        }));
    }
    let v_lit = js_string_literal(variant);
    let script = format!(
        r#"(() => {{
            const btn = document.querySelector('[data-aijia-hire-button="' + {v} + '"]');
            if (!btn) return {{ok: false, reason: 'hire_button_not_found', variant: {v}}};
            btn.click();
            return {{ok: true, variant: {v}}};
        }})()"#,
        v = v_lit,
    );
    eval_json(client, &script, window).await
}

async fn hire_wait(client: &mut Client, timeout_sec: u64, window: Option<&str>) -> Result<Value> {
    let probe = r"(() => {
        const w = document.querySelector('[data-aijia-hire-wizard]');
        if (!w) return {ready: false};
        return {ready: true, step: parseInt(w.getAttribute('data-aijia-hire-step') || '1', 10)};
    })()";
    let last = wait_until(client, probe, timeout_sec * 1000, window).await?;
    if last.get("ready").and_then(Value::as_bool) == Some(true) {
        return Ok(json!({
            "ok": true,
            "step": last.get("step").cloned().unwrap_or(Value::Null),
        }));
    }
    Ok(json!({
        "ok": false,
        "reason": "timeout",
        "timeoutSec": timeout_sec,
    }))
}

async fn hire_select_template(
    client: &mut Client,
    id: Option<&str>,
    name: Option<&str>,
    window: Option<&str>,
) -> Result<Value> {
    if id.is_none() && name.is_none() {
        return Ok(json!({
            "ok": false,
            "reason": "missing_selector",
            "hint": "pass --id or --name",
        }));
    }
    let id_lit = match id {
        Some(s) => js_string_literal(s),
        None => "null".to_string(),
    };
    let name_lit = match name {
        Some(s) => js_string_literal(s),
        None => "null".to_string(),
    };
    let script = format!(
        r#"(() => {{
            const wizard = document.querySelector('[data-aijia-hire-wizard]');
            if (!wizard) return {{ok: false, reason: 'wizard_not_open'}};
            const cards = [...wizard.querySelectorAll('[data-aijia-hire-template]')];
            if (cards.length === 0) return {{ok: false, reason: 'no_templates_rendered'}};
            const wantId = {id};
            const wantName = {name};
            let target = null;
            if (wantId) {{
                target = cards.find(c => c.getAttribute('data-aijia-hire-template-id') === wantId);
            }} else if (wantName) {{
                target = cards.find(c => (c.getAttribute('data-aijia-hire-template-name') || '').includes(wantName));
            }}
            if (!target) return {{
                ok: false,
                reason: 'template_not_found',
                requestedId: wantId,
                requestedName: wantName,
                available: cards.map(c => ({{
                    id: c.getAttribute('data-aijia-hire-template-id'),
                    name: c.getAttribute('data-aijia-hire-template-name'),
                }})),
            }};
            target.click();
            return {{
                ok: true,
                templateId: target.getAttribute('data-aijia-hire-template-id'),
                name: target.getAttribute('data-aijia-hire-template-name'),
            }};
        }})()"#,
        id = id_lit,
        name = name_lit,
    );
    eval_json(client, &script, window).await
}

async fn hire_action(client: &mut Client, action: &str, window: Option<&str>) -> Result<Value> {
    let a_lit = js_string_literal(action);
    let script = format!(
        r#"(() => {{
            const wizard = document.querySelector('[data-aijia-hire-wizard]');
            if (!wizard) return {{ok: false, reason: 'wizard_not_open'}};
            const btn = wizard.querySelector('[data-aijia-hire-action="' + {a} + '"]');
            if (!btn) return {{ok: false, reason: 'action_button_not_found', action: {a}}};
            if (btn.disabled) return {{ok: false, reason: 'action_button_disabled', action: {a}}};
            btn.click();
            return {{ok: true, action: {a}}};
        }})()"#,
        a = a_lit,
    );
    eval_json(client, &script, window).await
}

async fn hire_fill(
    client: &mut Client,
    field: &str,
    value: &str,
    window: Option<&str>,
) -> Result<Value> {
    if !["name", "cron"].contains(&field) {
        return Ok(json!({
            "ok": false,
            "reason": "invalid_field",
            "expected": ["name", "cron"],
            "got": field,
        }));
    }
    let f_lit = js_string_literal(field);
    let v_lit = js_string_literal(value);
    let script = format!(
        r#"(() => {{
            const wizard = document.querySelector('[data-aijia-hire-wizard]');
            if (!wizard) return {{ok: false, reason: 'wizard_not_open'}};
            const el = wizard.querySelector('[data-aijia-hire-field="' + {f} + '"]');
            if (!el) return {{ok: false, reason: 'field_missing', field: {f}}};
            const proto = el.tagName === 'TEXTAREA'
                ? window.HTMLTextAreaElement.prototype
                : window.HTMLInputElement.prototype;
            const setter = Object.getOwnPropertyDescriptor(proto, 'value').set;
            setter.call(el, {v});
            el.dispatchEvent(new Event('input', {{bubbles: true}}));
            return {{ok: true, field: {f}, value: el.value}};
        }})()"#,
        f = f_lit,
        v = v_lit,
    );
    eval_json(client, &script, window).await
}

// ─── employee: extended ops ─────────────────────────────────────────────────

async fn employee_status(
    client: &mut Client,
    name: Option<&str>,
    id: Option<&str>,
    window: Option<&str>,
) -> Result<Value> {
    if name.is_none() && id.is_none() {
        return Ok(json!({
            "ok": false,
            "reason": "missing_argument",
            "hint": "pass exactly one of --name or --id",
        }));
    }
    let (selector_field, want_lit) = match (name, id) {
        (Some(n), None) => ("data-aijia-employee-name", js_string_literal(n)),
        (None, Some(i)) => ("data-aijia-employee-id", js_string_literal(i)),
        _ => unreachable!("clap conflicts_with prevents both"),
    };
    let script = format!(
        r#"(() => {{
            const field = {field};
            const want = {want};
            const card = [...document.querySelectorAll('[data-aijia-employee-card]')]
                .find(c => c.getAttribute(field) === want);
            if (!card) return {{
                ok: false,
                reason: 'employee_card_not_found',
                requested: want,
                searchedField: field,
                cards: [...document.querySelectorAll('[data-aijia-employee-card]')]
                    .map(c => ({{
                        id: c.getAttribute('data-aijia-employee-id'),
                        name: c.getAttribute('data-aijia-employee-name'),
                    }})),
            }};
            return {{
                ok: true,
                employeeId: card.getAttribute('data-aijia-employee-id'),
                name: card.getAttribute('data-aijia-employee-name'),
                status: card.getAttribute('data-aijia-employee-status'),
                cronEnabled: card.getAttribute('data-aijia-employee-cron-enabled'),
                dispatchDisabled: card.getAttribute('data-aijia-employee-dispatch-disabled') === 'true',
            }};
        }})()"#,
        field = js_string_literal(selector_field),
        want = want_lit,
    );
    eval_json(client, &script, window).await
}

async fn employee_drawer_action(
    client: &mut Client,
    action: &str,
    window: Option<&str>,
) -> Result<Value> {
    const VALID: &[&str] = &[
        "dispatch",
        "close",
        "view-chat",
        "stop",
        "edit-cron",
        "toggle-cron",
        "toggle-cron-badge",
        "add-cron-trigger",
        "config-resource",
        "fire",
    ];
    if !VALID.contains(&action) {
        return Ok(json!({
            "ok": false,
            "reason": "invalid_action",
            "expected": VALID,
            "got": action,
        }));
    }
    let a_lit = js_string_literal(action);
    let script = format!(
        r#"(() => {{
            const dr = document.querySelector('[data-aijia-employee-drawer]');
            if (!dr) return {{ok: false, reason: 'drawer_not_open'}};
            const btn = dr.querySelector('[data-aijia-employee-action="' + {a} + '"]');
            if (!btn) return {{ok: false, reason: 'action_button_not_found', action: {a}}};
            if (btn.disabled) return {{ok: false, reason: 'action_button_disabled', action: {a}}};
            btn.click();
            return {{ok: true, action: {a}}};
        }})()"#,
        a = a_lit,
    );
    eval_json(client, &script, window).await
}

async fn employee_card_toggle_cron(
    client: &mut Client,
    name: &str,
    window: Option<&str>,
) -> Result<Value> {
    let n_lit = js_string_literal(name);
    let script = format!(
        r#"(() => {{
            const want = {n};
            const card = [...document.querySelectorAll('[data-aijia-employee-card]')]
                .find(c => c.getAttribute('data-aijia-employee-name') === want);
            if (!card) return {{ok: false, reason: 'employee_card_not_found', requested: want}};
            const btn = card.querySelector('[data-aijia-employee-action="pause-cron"], [data-aijia-employee-action="resume-cron"]');
            if (!btn) return {{ok: false, reason: 'cron_toggle_missing', hint: 'employee may not have cron configured'}};
            const which = btn.getAttribute('data-aijia-employee-action');
            btn.click();
            return {{ok: true, clicked: which}};
        }})()"#,
        n = n_lit,
    );
    eval_json(client, &script, window).await
}

// ─── resource config form: atomic ops ────────────────────────────────────────

async fn resource_fill(
    client: &mut Client,
    field: &str,
    value: &str,
    row: Option<usize>,
    window: Option<&str>,
) -> Result<Value> {
    let f_lit = js_string_literal(field);
    let v_lit = js_string_literal(value);
    let row_lit = match row {
        Some(n) => n.to_string(),
        None => "null".to_string(),
    };
    let script = format!(
        r#"(() => {{
            const form = document.querySelector('[data-aijia-resource-form]');
            if (!form) return {{ok: false, reason: 'resource_form_not_open'}};
            const wantField = {f};
            const wantRow = {r};
            let scope = form;
            if (wantRow !== null) {{
                const rowEl = form.querySelector('[data-aijia-resource-row="' + wantRow + '"]');
                if (!rowEl) return {{
                    ok: false,
                    reason: 'row_not_found',
                    requestedRow: wantRow,
                    rowCount: form.querySelectorAll('[data-aijia-resource-row]').length,
                }};
                scope = rowEl;
            }}
            let el = scope.querySelector('[data-aijia-resource-field="' + wantField + '"]');
            // SchemaForm wraps its inputs in a div with the attribute; descend to the actual input.
            if (el && !('value' in el)) {{
                el = el.querySelector('input, textarea, select') || el;
            }}
            if (!el || !('value' in el)) {{
                return {{ok: false, reason: 'field_missing', field: wantField}};
            }}
            const tag = el.tagName;
            const proto = tag === 'TEXTAREA'
                ? window.HTMLTextAreaElement.prototype
                : tag === 'SELECT'
                    ? window.HTMLSelectElement.prototype
                    : window.HTMLInputElement.prototype;
            const setter = Object.getOwnPropertyDescriptor(proto, 'value').set;
            setter.call(el, {v});
            el.dispatchEvent(new Event(tag === 'SELECT' ? 'change' : 'input', {{bubbles: true}}));
            return {{ok: true, field: wantField, value: el.value, row: wantRow}};
        }})()"#,
        f = f_lit,
        v = v_lit,
        r = row_lit,
    );
    eval_json(client, &script, window).await
}

async fn resource_action(
    client: &mut Client,
    action: &str,
    row: Option<usize>,
    window: Option<&str>,
) -> Result<Value> {
    let a_lit = js_string_literal(action);
    let row_lit = match row {
        Some(n) => n.to_string(),
        None => "null".to_string(),
    };
    let script = format!(
        r#"(() => {{
            const form = document.querySelector('[data-aijia-resource-form]');
            if (!form) return {{ok: false, reason: 'resource_form_not_open'}};
            const wantAction = {a};
            const wantRow = {r};
            let scope = form;
            if (wantRow !== null) {{
                const rowEl = form.querySelector('[data-aijia-resource-row="' + wantRow + '"]');
                if (!rowEl) return {{ok: false, reason: 'row_not_found', requestedRow: wantRow}};
                scope = rowEl;
            }}
            const btn = scope.querySelector('[data-aijia-resource-action="' + wantAction + '"]');
            if (!btn) return {{ok: false, reason: 'action_button_not_found', action: wantAction}};
            if (btn.disabled) return {{ok: false, reason: 'action_button_disabled', action: wantAction}};
            btn.click();
            return {{ok: true, action: wantAction, row: wantRow}};
        }})()"#,
        a = a_lit,
        r = row_lit,
    );
    eval_json(client, &script, window).await
}

// ─── workspace picker (home composer) ────────────────────────────────────────
//
// 产品契约：用户只在「新建对话」流程里挑一次 workspace。HomeTaskComposerCard
// 底部 dropdown 三种来源（recent / default / other）。这里不暴露切换 / 撤销 /
// 列表——产品没有这些 UI（已确认 `revokeAuthorizedWorkspace` 无 caller，
// ChatTopBar 只读展示）。`other` 走 OS folder dialog，dev 下由
// `workspace-queue-path` 入队 mock 跳过 dialog。

async fn workspace_queue_path(
    client: &mut Client,
    path: &str,
    window: Option<&str>,
) -> Result<Value> {
    let path_lit = js_string_literal(path);
    let script = format!(
        r#"(() => {{
            const aj = window.__aijia;
            if (!aj) return {{ok: false, reason: 'dev_hooks_unavailable'}};
            if (!Array.isArray(aj._pickDirectoryMockQueue)) {{
                aj._pickDirectoryMockQueue = [];
            }}
            aj._pickDirectoryMockQueue.push({p});
            return {{
                ok: true,
                queued: {p},
                queueDepth: aj._pickDirectoryMockQueue.length,
            }};
        }})()"#,
        p = path_lit,
    );
    eval_json(client, &script, window).await
}

async fn workspace_open_picker(client: &mut Client, window: Option<&str>) -> Result<Value> {
    let script = format!(
        r#"(() => {{
            {helper}
            const btn = document.querySelector('[data-aijia-workspace-trigger]');
            if (!btn) return {{ok: false, reason: 'workspace_trigger_not_found'}};
            if (btn.disabled) return {{ok: false, reason: 'workspace_trigger_disabled'}};
            __aijia_radixDropdownClick(btn);
            return {{ok: true}};
        }})()"#,
        helper = RADIX_DROPDOWN_CLICK_HELPER_JS,
    );
    eval_json(client, &script, window).await
}

async fn workspace_pick(
    client: &mut Client,
    variant: &str,
    path: Option<&str>,
    window: Option<&str>,
) -> Result<Value> {
    match variant {
        "default" | "other" => {
            let action = if variant == "default" { "pick-default" } else { "pick-other" };
            let a_lit = js_string_literal(action);
            // Radix DropdownMenuContent renders in a portal — selector lookup
            // is global. 不限定到具体 menu container 因为打开后只可能存在一个。
            let script = format!(
                r#"(() => {{
                    const item = document.querySelector('[data-aijia-workspace-action="' + {a} + '"]');
                    if (!item) return {{ok: false, reason: 'item_not_found', action: {a}, hint: 'workspace dropdown 未打开?'}};
                    item.click();
                    return {{ok: true, action: {a}}};
                }})()"#,
                a = a_lit,
            );
            eval_json(client, &script, window).await
        }
        "recent" => {
            let p = match path {
                Some(s) => s,
                None => {
                    return Ok(json!({
                        "ok": false,
                        "reason": "missing_path",
                        "hint": "--variant recent 必须配合 --path <absolute path>",
                    }));
                }
            };
            let p_lit = js_string_literal(p);
            let script = format!(
                r#"(() => {{
                    const want = {p};
                    const items = [...document.querySelectorAll('[data-aijia-workspace-recent]')];
                    const item = items.find(el => el.getAttribute('data-aijia-workspace-path') === want);
                    if (!item) return {{
                        ok: false,
                        reason: 'recent_path_not_found',
                        requested: want,
                        available: items.map(el => el.getAttribute('data-aijia-workspace-path')),
                    }};
                    item.click();
                    return {{ok: true, path: want}};
                }})()"#,
                p = p_lit,
            );
            eval_json(client, &script, window).await
        }
        _ => Ok(json!({
            "ok": false,
            "reason": "invalid_variant",
            "expected": ["default", "other", "recent"],
            "got": variant,
        })),
    }
}

// ─── expert teams: start by click ────────────────────────────────────────────
//
// 产品契约：专家团是静态启动器，没有 CRUD。点 card 直接 createConversation +
// setExpertTeam (localStorage) + 跳 chat 页。"派活" = "点 card" 一个原子。

async fn expert_team_start(
    client: &mut Client,
    name: &str,
    window: Option<&str>,
) -> Result<Value> {
    let n_lit = js_string_literal(name);
    let script = format!(
        r#"(() => {{
            const want = {n};
            const cards = [...document.querySelectorAll('[data-aijia-expert-team-card]')];
            const card = cards.find(c => c.getAttribute('data-aijia-expert-team-name') === want);
            if (!card) return {{
                ok: false,
                reason: 'expert_team_card_not_found',
                requested: want,
                available: cards.map(c => ({{
                    id: c.getAttribute('data-aijia-expert-team-id'),
                    name: c.getAttribute('data-aijia-expert-team-name'),
                }})),
            }};
            card.click();
            return {{
                ok: true,
                teamId: card.getAttribute('data-aijia-expert-team-id'),
                name: want,
            }};
        }})()"#,
        n = n_lit,
    );
    eval_json(client, &script, window).await
}

// ─── skill-center: import flow + cards ──────────────────────────────────────

async fn skill_import_queue(
    client: &mut Client,
    path: &str,
    window: Option<&str>,
) -> Result<Value> {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return Ok(json!({
            "ok": false,
            "reason": "empty_path",
            "hint": "pass --path as a single absolute path (folder or .zip)",
        }));
    }
    let p_lit = js_string_literal(trimmed);
    let script = format!(
        r#"(() => {{
            const aj = window.__aijia;
            if (!aj) return {{ok: false, reason: 'dev_hooks_unavailable'}};
            if (!Array.isArray(aj._pickSkillImportMockQueue)) {{
                aj._pickSkillImportMockQueue = [];
            }}
            aj._pickSkillImportMockQueue.push({p});
            return {{
                ok: true,
                queueDepth: aj._pickSkillImportMockQueue.length,
                queued: {p},
            }};
        }})()"#,
        p = p_lit,
    );
    eval_json(client, &script, window).await
}

async fn skill_import_open(client: &mut Client, window: Option<&str>) -> Result<Value> {
    let script = format!(
        r#"(() => {{
            {helper}
            const btn = document.querySelector('[data-aijia-skill-import-trigger]');
            if (!btn) return {{ok: false, reason: 'skill_import_trigger_not_found'}};
            if (btn.disabled) return {{ok: false, reason: 'skill_import_trigger_disabled'}};
            __aijia_radixDropdownClick(btn);
            return {{ok: true}};
        }})()"#,
        helper = RADIX_DROPDOWN_CLICK_HELPER_JS,
    );
    eval_json(client, &script, window).await
}

async fn skill_import_pick(
    client: &mut Client,
    variant: &str,
    window: Option<&str>,
) -> Result<Value> {
    let v = variant.trim();
    if v != "directory" && v != "archive" {
        return Ok(json!({
            "ok": false,
            "reason": "invalid_variant",
            "hint": "--variant must be one of: directory, archive",
            "received": variant,
        }));
    }
    let v_lit = js_string_literal(v);
    let script = format!(
        r#"(() => {{
            const want = {v};
            const item = document.querySelector(`[data-aijia-skill-import-action="${{want}}"]`);
            if (!item) return {{
                ok: false,
                reason: 'skill_import_action_item_not_found',
                requested: want,
                hint: 'caller must `skill-import-open` first to mount the dropdown menu',
                available: [...document.querySelectorAll('[data-aijia-skill-import-action]')]
                    .map(el => el.getAttribute('data-aijia-skill-import-action')),
            }};
            item.click();
            return {{ok: true, variant: want}};
        }})()"#,
        v = v_lit,
    );
    eval_json(client, &script, window).await
}

async fn skill_cards(client: &mut Client, window: Option<&str>) -> Result<Value> {
    let script = r#"(() => {
        const cards = [...document.querySelectorAll('[data-aijia-skill-card]')];
        return {
            ok: true,
            count: cards.length,
            cards: cards.map(c => ({
                id: c.getAttribute('data-aijia-skill-id') || null,
                source: c.getAttribute('data-aijia-skill-source') || null,
            })),
        };
    })()"#;
    eval_json(client, script, window).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn js_string_literal_escapes_quotes_and_backslash() {
        assert_eq!(js_string_literal("a'b"), r"'a\'b'");
        assert_eq!(js_string_literal(r"a\b"), r"'a\\b'");
    }

    #[test]
    fn js_string_literal_escapes_control_chars() {
        assert_eq!(js_string_literal("a\nb"), r"'a\nb'");
        assert_eq!(js_string_literal("a\tb"), r"'a\tb'");
    }

    #[test]
    fn js_string_literal_preserves_unicode_text() {
        // CJK passes through as-is — the surrounding single quotes are still
        // a single JS string literal; no extra escaping needed.
        assert_eq!(js_string_literal("你好"), "'你好'");
    }

    #[test]
    fn js_string_literal_escapes_line_separators() {
        // U+2028 / U+2029 terminate JS string literals; must be encoded.
        let s = format!("a{}b", '\u{2028}');
        assert_eq!(js_string_literal(&s), "'a\\u2028b'");
    }
}
