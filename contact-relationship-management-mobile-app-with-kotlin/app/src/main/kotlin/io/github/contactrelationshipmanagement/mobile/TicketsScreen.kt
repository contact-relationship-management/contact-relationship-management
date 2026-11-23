package io.github.contactrelationshipmanagement.mobile

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.unit.dp
import io.github.contactrelationshipmanagement.mobile.core.Ticket
import io.github.lilydesignsystem.compose.BadgeKind
import io.github.lilydesignsystem.compose.LilyBadge

@Composable
fun TicketsScreen(model: CrmViewModel) {
    Loading(model.tickets, model::loadTickets) { tickets ->
        LazyColumn(contentPadding = PaddingValues(16.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
            if (tickets.isEmpty()) item { Text(stringResource(R.string.tickets_empty)) }
            items(tickets, key = { it.pid }) { ticket ->
                TicketRow(ticket)
                HorizontalDivider()
            }
        }
    }
}

/** A ticket's title, status and priority, and a badge (words, not only colour; MOB-10) for each breached deadline. */
@Composable
fun TicketRow(ticket: Ticket) {
    Column(Modifier.padding(vertical = 4.dp), verticalArrangement = Arrangement.spacedBy(4.dp)) {
        Text(ticket.title, style = MaterialTheme.typography.titleMedium)
        Row(horizontalArrangement = Arrangement.spacedBy(12.dp)) {
            Text("${stringResource(R.string.ticket_status)}: ${ticket.status}", style = MaterialTheme.typography.bodySmall)
            Text("${stringResource(R.string.ticket_priority)}: ${ticket.priority}", style = MaterialTheme.typography.bodySmall)
        }
        val sla = stringResource(R.string.ticket_sla)
        Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            if (ticket.firstResponseBreached) {
                LilyBadge(stringResource(R.string.ticket_first_response_breached), kind = BadgeKind.Error, label = "$sla: ${stringResource(R.string.ticket_first_response_breached)}")
            }
            if (ticket.resolutionBreached) {
                LilyBadge(stringResource(R.string.ticket_resolution_breached), kind = BadgeKind.Error, label = "$sla: ${stringResource(R.string.ticket_resolution_breached)}")
            }
        }
    }
}
