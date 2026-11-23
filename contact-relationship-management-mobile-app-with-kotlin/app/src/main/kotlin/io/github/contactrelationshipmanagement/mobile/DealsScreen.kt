package io.github.contactrelationshipmanagement.mobile

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.pluralStringResource
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.heading
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.dp
import io.github.contactrelationshipmanagement.mobile.core.Deal
import io.github.lilydesignsystem.compose.BadgeKind
import io.github.lilydesignsystem.compose.LilyBadge
import io.github.lilydesignsystem.compose.LilyCard

@Composable
fun DealsScreen(model: CrmViewModel) {
    Loading(model.deals, model::loadDeals) { grouped ->
        LazyColumn(contentPadding = PaddingValues(16.dp), verticalArrangement = Arrangement.spacedBy(16.dp)) {
            if (grouped.boards.all { board -> board.columns.all { it.deals.isEmpty() } } && grouped.unplaced.isEmpty()) {
                item { Text(stringResource(R.string.deals_empty)) }
            }
            for (board in grouped.boards) {
                item(key = board.pipeline.pid) {
                    Text(board.pipeline.name, style = MaterialTheme.typography.titleLarge, modifier = Modifier.semantics { heading() })
                }
                for (column in board.columns) {
                    item(key = column.stage.pid) {
                        LilyCard(modifier = Modifier.fillMaxWidth(), heading = pluralStringResource(R.plurals.deals_in_stage, column.deals.size, column.stage.name, column.deals.size)) {
                            if (column.stage.isWon) LilyBadge(stringResource(R.string.deal_won), kind = BadgeKind.Success)
                            if (column.stage.isLost) LilyBadge(stringResource(R.string.deal_lost), kind = BadgeKind.Warning)
                            for (deal in column.deals) DealRow(deal)
                        }
                    }
                }
            }
            if (grouped.unplaced.isNotEmpty()) {
                item {
                    LilyCard(modifier = Modifier.fillMaxWidth(), heading = stringResource(R.string.deals_unplaced)) { for (deal in grouped.unplaced) DealRow(deal) }
                }
            }
        }
    }
}

@Composable
private fun DealRow(deal: Deal) {
    Column(Modifier.padding(vertical = 4.dp).semantics(mergeDescendants = true) {}) {
        Text(deal.name, style = MaterialTheme.typography.labelLarge)
        Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            Text(stringResource(R.string.deal_amount), style = MaterialTheme.typography.bodySmall)
            Text(amount(deal.exactMinorText, deal.currency))
        }
        deal.expectedCloseOn?.let {
            Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                Text(stringResource(R.string.deal_close), style = MaterialTheme.typography.bodySmall)
                Text(it)
            }
        }
    }
}
