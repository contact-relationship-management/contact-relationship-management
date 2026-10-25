<!--
  Workflow rules (`/workflows`): a trigger, conditions in the segment filter language, and a short
  list of actions from a closed set (CRM-R40). Nothing here runs code or calls a web address. A dry
  run shows what a rule would do to the current data, and the run log says why each run happened.
-->
<script lang="ts">
  import { errorMessage } from "#lib/errors.js";
  import { resource } from "#lib/resource.svelte.js";
  import {
    deleteWorkflow,
    dryRunWorkflow,
    listWorkflows,
    replaceWorkflow,
    saveWorkflow,
    workflowRuns,
    type Workflow,
    type WorkflowAction,
    type WorkflowDryRun,
    type WorkflowRun,
  } from "#lib/api/crm.js";
  import { t } from "#lib/i18n.svelte.js";

  const list = resource(listWorkflows);
  const workflows = $derived(list.value ?? []);

  const TRIGGERS = [
    "contact_created",
    "contact_updated",
    "deal_stage_changed",
    "schedule",
    "lead_created",
  ] as const;
  const TRIGGER_LABEL = {
    contact_created: "wf.trigger.contact_created",
    contact_updated: "wf.trigger.contact_updated",
    deal_stage_changed: "wf.trigger.deal_stage_changed",
    schedule: "wf.trigger.schedule",
    lead_created: "wf.trigger.lead_created",
  } as const;
  const ACTION_LABEL = {
    create_task: "wf.action.create_task",
    notify: "wf.action.notify",
    set_field: "wf.action.set_field",
    enrol_nurture: "wf.action.enrol_nurture",
    send_webhook: "wf.action.send_webhook",
    assign_owner: "wf.action.assign_owner",
  } as const;
  // A lead is assigned, not enrolled or edited; a contact is not assigned.
  const LEAD_ACTIONS = ["create_task", "notify", "assign_owner", "send_webhook"];
  const CONTACT_ACTIONS = ["create_task", "notify", "set_field", "enrol_nurture", "send_webhook"];
  const LEAD_CONDITIONS = [
    {
      key: "sources",
      label: "wf.cond.sources",
      values: ["web", "referral", "event", "import", "campaign"],
    },
  ] as const;
  const CONDITIONS = [
    { key: "statuses", label: "wf.cond.statuses", values: ["active", "inactive"] },
    { key: "channels", label: "wf.cond.channels", values: ["email", "phone", "post", "none"] },
    {
      key: "account_tiers",
      label: "wf.cond.account_tiers",
      values: ["prospect", "customer", "partner", "former"],
    },
  ] as const;
  const FIELDS = ["job_title", "owner_ref", "preferred_channel", "status"];

  let editing = $state<Workflow | null>(null);
  let creating = $state(false);
  let name = $state("");
  let enabled = $state(true);
  let trigger = $state<string>("contact_created");
  let every = $state(60);
  const allowed = $derived(trigger === "lead_created" ? LEAD_ACTIONS : CONTACT_ACTIONS);
  let conditions = $state({
    statuses: [] as string[],
    channels: [] as string[],
    account_tiers: [] as string[],
    sources: [] as string[],
  });
  let actions = $state<WorkflowAction[]>([]);
  let adding = $state<WorkflowAction["type"]>("create_task");
  let error = $state<string | null>(null);
  let dry = $state<WorkflowDryRun | null>(null);
  let runs = $state<WorkflowRun[]>([]);

  const open = $derived(creating || editing !== null);

  function start(workflow: Workflow | null) {
    editing = workflow;
    creating = workflow === null;
    name = workflow?.name ?? "";
    enabled = workflow?.enabled ?? true;
    trigger = workflow?.trigger ?? "contact_created";
    every = workflow?.every_minutes ?? 60;
    conditions = {
      statuses: [...(workflow?.conditions.statuses ?? [])],
      channels: [...(workflow?.conditions.channels ?? [])],
      account_tiers: [...(workflow?.conditions.account_tiers ?? [])],
      sources: [...(workflow?.conditions.sources ?? [])],
    };
    actions = structuredClone($state.snapshot(workflow?.actions ?? []));
    error = null;
    dry = null;
    runs = [];
    if (workflow) void loadRuns(workflow.pid);
  }

  async function loadRuns(pid: string) {
    try {
      runs = await workflowRuns(pid);
    } catch {
      runs = [];
    }
  }

  function toggle(
    key: "statuses" | "channels" | "account_tiers" | "sources",
    value: string,
  ) {
    const now = conditions[key];
    conditions[key] = now.includes(value) ? now.filter((v) => v !== value) : [...now, value];
  }

  function add() {
    const made: Record<WorkflowAction["type"], WorkflowAction> = {
      create_task: { type: "create_task", summary: "", assignee_ref: null, due_in_days: null },
      notify: { type: "notify", recipient_ref: "" },
      set_field: { type: "set_field", field: "job_title", value: "" },
      enrol_nurture: { type: "enrol_nurture", sequence_pid: "" },
      send_webhook: { type: "send_webhook", event: "" },
      assign_owner: { type: "assign_owner", pool: [], strategy: "round_robin", notify: true },
    };
    actions = [...actions, made[adding]];
  }

  const blankToNull = (v: string | null | undefined) => (v && v.trim() ? v.trim() : null);

  function body() {
    return {
      name,
      enabled,
      trigger,
      every_minutes: trigger === "schedule" ? every : null,
      conditions: $state.snapshot(conditions),
      actions: $state.snapshot(actions).map((a): WorkflowAction => {
        if (a.type === "create_task")
          return {
            ...a,
            assignee_ref: blankToNull(a.assignee_ref),
            due_in_days: a.due_in_days ? Number(a.due_in_days) : null,
          };
        if (a.type === "set_field") return { ...a, value: blankToNull(a.value) };
        return a;
      }),
    };
  }

  async function save() {
    error = null;
    try {
      if (editing) await replaceWorkflow(editing.pid, body());
      else await saveWorkflow(body());
      creating = false;
      editing = null;
      await list.reload();
    } catch (cause) {
      error = errorMessage(cause);
    }
  }

  async function remove() {
    if (!editing) return;
    error = null;
    try {
      await deleteWorkflow(editing.pid);
      editing = null;
      creating = false;
      await list.reload();
    } catch (cause) {
      error = errorMessage(cause);
    }
  }

  async function tryIt() {
    if (!editing) return;
    error = null;
    try {
      dry = await dryRunWorkflow(editing.pid);
    } catch (cause) {
      error = errorMessage(cause);
    }
  }
</script>

<h1>{t("wf.title")}</h1>
<p class="muted">{t("wf.hint")}</p>

<p><button onclick={() => start(null)} data-testid="workflow-new">{t("wf.new")}</button></p>

<ul data-testid="workflow-list">
  {#each workflows as workflow (workflow.pid)}
    <li>
      <button onclick={() => start(workflow)}>{workflow.name}</button>
      <span class="chip">{t(TRIGGER_LABEL[workflow.trigger as keyof typeof TRIGGER_LABEL])}</span>
      {#if !workflow.enabled}<span class="muted">—</span>{/if}
    </li>
  {/each}
</ul>

{#if open}
  <form
    class="panel"
    data-testid="workflow-form"
    onsubmit={(e) => {
      e.preventDefault();
      void save();
    }}
  >
    <label>
      {t("wf.name")}
      <input bind:value={name} required data-testid="workflow-name" />
    </label>
    <label><input type="checkbox" bind:checked={enabled} /> {t("wf.enabled")}</label>
    <label>
      {t("wf.trigger")}
      <select bind:value={trigger} data-testid="workflow-trigger">
        {#each TRIGGERS as option (option)}
          <option value={option}>{t(TRIGGER_LABEL[option])}</option>
        {/each}
      </select>
    </label>
    {#if trigger === "schedule"}
      <label>
        {t("wf.every")}
        <input type="number" min="5" max="43200" bind:value={every} />
      </label>
    {/if}

    <fieldset>
      <legend>{t("wf.conditions")}</legend>
      {#each trigger === "lead_created" ? LEAD_CONDITIONS : CONDITIONS as group (group.key)}
        <span>
          {t(group.label)}:
          {#each group.values as value (value)}
            <label>
              <input
                type="checkbox"
                checked={conditions[group.key].includes(value)}
                onchange={() => toggle(group.key, value)}
              />
              {value}
            </label>
          {/each}
        </span>
        <br />
      {/each}
    </fieldset>

    <fieldset>
      <legend>{t("wf.actions")}</legend>
      <ol data-testid="workflow-actions">
        {#each actions as action, i (i)}
          <li>
            <strong>{t(ACTION_LABEL[action.type])}</strong>
            {#if action.type === "create_task"}
              <label>{t("wf.summary")} <input bind:value={action.summary} /></label>
              <label>{t("wf.assignee")} <input bind:value={action.assignee_ref} /></label>
              <label>{t("wf.dueIn")} <input type="number" min="0" bind:value={action.due_in_days} /></label>
            {:else if action.type === "notify"}
              <label>{t("wf.recipient")} <input bind:value={action.recipient_ref} /></label>
            {:else if action.type === "set_field"}
              <label>
                {t("wf.field")}
                <select bind:value={action.field}>
                  {#each FIELDS as field (field)}<option value={field}>{field}</option>{/each}
                </select>
              </label>
              <label>{t("wf.value")} <input bind:value={action.value} /></label>
            {:else if action.type === "enrol_nurture"}
              <label>{t("wf.sequence")} <input bind:value={action.sequence_pid} /></label>
            {:else if action.type === "assign_owner"}
              <label>
                {t("wf.pool")}
                <input
                  value={action.pool.join(", ")}
                  oninput={(e) =>
                    (action.pool = e.currentTarget.value
                      .split(",")
                      .map((w) => w.trim())
                      .filter(Boolean))}
                />
              </label>
              <label>
                {t("wf.strategy")}
                <select bind:value={action.strategy}>
                  <option value="round_robin">{t("wf.strategy.round_robin")}</option>
                  <option value="least_open">{t("wf.strategy.least_open")}</option>
                </select>
              </label>
              <label><input type="checkbox" bind:checked={action.notify} /> {t("wf.notifyOwner")}</label>
            {:else}
              <label>{t("wf.event")} <input bind:value={action.event} /></label>
            {/if}
            <button type="button" onclick={() => (actions = actions.filter((_, at) => at !== i))}>
              {t("wf.removeAction")}
            </button>
          </li>
        {/each}
      </ol>
      <select bind:value={adding} data-testid="workflow-action-type">
        {#each allowed as type (type)}
          <option value={type}>{t(ACTION_LABEL[type as keyof typeof ACTION_LABEL])}</option>
        {/each}
      </select>
      <button type="button" onclick={add} data-testid="workflow-add-action">{t("wf.addAction")}</button>
    </fieldset>

    {#if error}<p class="error" data-testid="workflow-error">{error}</p>{/if}
    <button type="submit" data-testid="workflow-save">{t("wf.save")}</button>
    {#if editing}
      <button type="button" onclick={() => void tryIt()} data-testid="workflow-dry">{t("wf.dryRun")}</button>
      <button type="button" onclick={() => void remove()}>{t("wf.delete")}</button>
    {/if}
  </form>

  {#if dry}
    <div class="panel" data-testid="workflow-dry-result">
      <p>
        {t("wf.examined")}: <strong>{dry.examined}</strong> · {t("wf.wouldFire")}:
        <strong data-testid="workflow-would-fire">{dry.would_fire}</strong>
      </p>
      {#each dry.samples as sample (sample.subject_pid)}
        <p class="muted">
          {#each sample.steps as step, i (i)}
            {step.action}: <span class="chip">{step.result}</span>
            {#if step.detail}({step.detail}){/if}
            {" "}
          {/each}
        </p>
      {/each}
    </div>
  {/if}

  {#if editing}
    <h2>{t("wf.runs")}</h2>
    {#if runs.length === 0}
      <p class="muted" data-testid="workflow-no-runs">{t("wf.noRuns")}</p>
    {:else}
      <ul data-testid="workflow-runs">
        {#each runs as run (run.pid)}
          <li>
            <span class="chip">{run.outcome}</span>
            {run.reason}
            <br />
            <span class="muted small">
              {#each run.steps as step, i (i)}
                {step.action}: {step.result}{step.detail ? ` (${step.detail})` : ""}{i < run.steps.length - 1 ? " · " : ""}
              {/each}
            </span>
          </li>
        {/each}
      </ul>
    {/if}
  {/if}
{/if}
