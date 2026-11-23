package io.github.contactrelationshipmanagement.mobile.core

import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable
import kotlinx.serialization.json.Json

/** The service's JSON, as the OpenAPI document names it. Members the app does not show are ignored when read. */
public val ApiJson: Json = Json {
    ignoreUnknownKeys = true
    explicitNulls = false
}

@Serializable
public data class Contact(
    val pid: String,
    @SerialName("display_name") val displayName: String,
    @SerialName("person_ref") val personRef: String,
    @SerialName("job_title") val jobTitle: String? = null,
    val status: String,
    @SerialName("marketing_consent") val marketingConsent: String,
    @SerialName("preferred_channel") val preferredChannel: String,
    @SerialName("account_pid") val accountPid: String? = null,
)

@Serializable
public data class Activity(
    val pid: String,
    val kind: String,
    val summary: String,
    val done: Boolean,
    @SerialName("occurred_at") val occurredAt: String,
    @SerialName("due_on") val dueOn: String? = null,
    @SerialName("subject_kind") val subjectKind: String,
    @SerialName("subject_pid") val subjectPid: String,
)

/** A deal as `GET /api/deals` lists it, and as a contact's detail carries it (without the exact text). */
@Serializable
public data class Deal(
    val pid: String,
    val name: String,
    val currency: String,
    @SerialName("amount_minor") val amountMinor: Long? = null,
    @SerialName("amount_minor_text") val amountMinorText: String? = null,
    @SerialName("pipeline_pid") val pipelinePid: String,
    @SerialName("stage_pid") val stagePid: String,
    @SerialName("kanban_position") val kanbanPosition: Int,
    @SerialName("expected_close_on") val expectedCloseOn: String? = null,
    val won: Boolean,
    @SerialName("closed_at") val closedAt: String? = null,
) {
    /**
     * The exact amount text (MOB-6): `amount_minor_text` where the service sends it; else the `int64`, which a JVM
     * `Long` holds exactly. Null when masked.
     */
    val exactMinorText: String? get() = amountMinorText ?: amountMinor?.toString()
}

@Serializable
public data class Ticket(
    val pid: String,
    val title: String,
    val status: String,
    val priority: String,
    val channel: String,
    @SerialName("opened_at") val openedAt: String,
    @SerialName("first_response_breached") val firstResponseBreached: Boolean,
    @SerialName("resolution_breached") val resolutionBreached: Boolean,
    @SerialName("contact_pid") val contactPid: String? = null,
)

@Serializable
public data class ContactDetail(
    val contact: Contact,
    val activities: List<Activity>,
    val deals: List<Deal>,
    val tickets: List<Ticket>,
)

@Serializable
public data class PipelineRecord(val pid: String, val name: String)

@Serializable
public data class StageRecord(
    val pid: String,
    val name: String,
    @SerialName("pipeline_pid") val pipelinePid: String,
    val position: Int,
    @SerialName("is_won") val isWon: Boolean,
    @SerialName("is_lost") val isLost: Boolean,
    @SerialName("probability_percent") val probabilityPercent: Int,
)

@Serializable
public data class Pipeline(val pipeline: PipelineRecord, val stages: List<StageRecord>)

/** The activity kinds the service accepts (`ACTIVITY_KINDS` in the domain crate). */
public val ACTIVITY_KINDS: List<String> = listOf("call", "email", "meeting", "note", "task")

/** `POST /api/activities` body, about a contact. */
@Serializable
public data class ActivityPayload(
    val kind: String,
    val summary: String,
    @SerialName("subject_kind") val subjectKind: String,
    @SerialName("subject_pid") val subjectPid: String,
)
