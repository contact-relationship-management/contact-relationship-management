package io.github.contactrelationshipmanagement.mobile

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.selection.selectable
import androidx.compose.foundation.selection.selectableGroup
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Button
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.RadioButton
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.focus.focusRequester
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.unit.dp
import io.github.contactrelationshipmanagement.mobile.core.ACTIVITY_KINDS
import io.github.contactrelationshipmanagement.mobile.core.ContactDetail
import io.github.lilydesignsystem.compose.AlertKind
import io.github.lilydesignsystem.compose.ErrorSummary
import io.github.lilydesignsystem.compose.Field
import io.github.lilydesignsystem.compose.InsetText
import io.github.lilydesignsystem.compose.LilyAlert
import io.github.lilydesignsystem.compose.LilyBadge
import io.github.lilydesignsystem.compose.LilyCard
import io.github.lilydesignsystem.compose.Problem as LilyProblem
import io.github.lilydesignsystem.compose.SummaryList
import io.github.lilydesignsystem.compose.SummaryListItem
import io.github.lilydesignsystem.compose.Tag

@Composable
fun ContactScreen(model: CrmViewModel, pid: String) {
    Loading(model.detail, { model.loadDetail(pid) }) { detail ->
        if (detail.contact.pid == pid) ContactDetailView(model, detail)
    }
}

@Composable
private fun ContactDetailView(model: CrmViewModel, detail: ContactDetail) {
    val contact = detail.contact
    val none = stringResource(R.string.none)
    Column(ScreenPadding.verticalScroll(rememberScrollState()), verticalArrangement = Arrangement.spacedBy(16.dp)) {
        Text(contact.displayName, style = MaterialTheme.typography.headlineSmall)
        SummaryList(label = stringResource(R.string.contact_details)) {
            SummaryListItem(stringResource(R.string.contact_job_title)) { Text(contact.jobTitle ?: none) }
            SummaryListItem(stringResource(R.string.contact_status)) { Text(contact.status) }
            SummaryListItem(stringResource(R.string.contact_consent)) {
                val consent = Messages.consent(contact.marketingConsent)?.let { stringResource(it) } ?: contact.marketingConsent
                LilyBadge(consent, kind = Messages.consentKind(contact.marketingConsent))
            }
            SummaryListItem(stringResource(R.string.contact_channel)) { Text(contact.preferredChannel) }
        }
        // MOB-12: consent is shown here, and changed in the web app.
        InsetText { Text(stringResource(R.string.contact_consent_note)) }
        Button(onClick = { model.openLog(contact.pid) }) { Text(stringResource(R.string.log_activity)) }

        LilyCard(modifier = Modifier.fillMaxWidth(), heading = stringResource(R.string.contact_activities)) {
            if (detail.activities.isEmpty()) Text(none)
            for (activity in detail.activities) {
                Column(Modifier.padding(vertical = 4.dp)) {
                    val kind = Messages.kind(activity.kind)?.let { stringResource(it) } ?: activity.kind
                    Row(horizontalArrangement = Arrangement.spacedBy(8.dp), verticalAlignment = Alignment.CenterVertically) {
                        Text(kind, style = MaterialTheme.typography.labelLarge)
                        Tag(
                            stringResource(if (activity.done) R.string.activity_done else R.string.activity_open),
                            label = kind,
                        )
                    }
                    Text(activity.summary)
                    Text(activity.occurredAt, style = MaterialTheme.typography.bodySmall)
                }
            }
        }
        LilyCard(modifier = Modifier.fillMaxWidth(), heading = stringResource(R.string.contact_deals)) {
            if (detail.deals.isEmpty()) Text(none)
            for (deal in detail.deals) {
                Text(deal.name, style = MaterialTheme.typography.labelLarge)
                Text(amount(deal.exactMinorText, deal.currency))
            }
        }
        LilyCard(modifier = Modifier.fillMaxWidth(), heading = stringResource(R.string.contact_tickets)) {
            if (detail.tickets.isEmpty()) Text(none)
            for (ticket in detail.tickets) TicketRow(ticket)
        }
    }
}

@Composable
fun LogActivityScreen(model: CrmViewModel, pid: String) {
    val form = model.logForm
    val summaryFocus = remember { FocusRequester() }
    val summaryError = if (form.summaryMissing) stringResource(R.string.log_summary_required) else null
    Column(ScreenPadding.verticalScroll(rememberScrollState()), verticalArrangement = Arrangement.spacedBy(16.dp)) {
        if (summaryError != null) {
            ErrorSummary(stringResource(R.string.problem_title), listOf(LilyProblem(summaryError) { summaryFocus.requestFocus() }))
        }
        form.error?.let { LilyAlert(text(it), kind = AlertKind.Error) }
        if (form.sent) LilyAlert(stringResource(R.string.log_sent), kind = AlertKind.Success)

        Text(stringResource(R.string.log_kind), style = MaterialTheme.typography.titleSmall)
        Column(Modifier.selectableGroup()) {
            for (kind in ACTIVITY_KINDS) {
                val label = Messages.kind(kind)?.let { stringResource(it) } ?: kind
                Row(
                    Modifier.fillMaxWidth()
                        .selectable(selected = form.kind == kind, role = Role.RadioButton) { model.setLogKind(kind) }
                        .padding(vertical = 8.dp),
                    verticalAlignment = Alignment.CenterVertically,
                ) {
                    RadioButton(selected = form.kind == kind, onClick = null)
                    Text(label, Modifier.padding(start = 12.dp))
                }
            }
        }
        Field(
            label = stringResource(R.string.log_summary),
            description = stringResource(R.string.log_summary_hint),
            error = summaryError,
            required = true,
            requiredLabel = stringResource(R.string.required),
        ) { semantics ->
            OutlinedTextField(
                form.summary,
                model::setLogSummary,
                isError = summaryError != null,
                minLines = 3,
                modifier = semantics.fillMaxWidth().focusRequester(summaryFocus),
            )
        }
        Button(onClick = { model.sendLog(pid) }, enabled = !form.sending) {
            Text(stringResource(if (form.error != null) R.string.retry else R.string.log_send))
        }
    }
}
