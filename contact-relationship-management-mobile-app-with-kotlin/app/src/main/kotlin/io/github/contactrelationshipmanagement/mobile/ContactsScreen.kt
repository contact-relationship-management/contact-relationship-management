package io.github.contactrelationshipmanagement.mobile

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.ui.semantics.Role
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.unit.dp
import io.github.lilydesignsystem.compose.AlertKind
import io.github.lilydesignsystem.compose.Field
import io.github.lilydesignsystem.compose.LilyAlert

@Composable
fun ContactsScreen(model: CrmViewModel) {
    Loading(model.contacts, model::loadContacts) { listed ->
        val shown = listed.rows.filter { it.displayName.contains(model.filter, ignoreCase = true) }
        LazyColumn(contentPadding = PaddingValues(16.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
            if (listed.servedWithoutToken) {
                item { LilyAlert(stringResource(R.string.demo_message), kind = AlertKind.Warning, heading = stringResource(R.string.demo_heading)) }
            }
            item {
                Field(label = stringResource(R.string.contacts_filter)) { semantics ->
                    OutlinedTextField(model.filter, { model.filter = it }, singleLine = true, modifier = semantics.fillMaxWidth())
                }
            }
            if (shown.isEmpty()) item { Text(stringResource(R.string.contacts_empty)) }
            items(shown, key = { it.pid }) { contact ->
                Column(
                    Modifier.fillMaxWidth()
                        .clickable(role = Role.Button) { model.openContact(contact.pid) }
                        .padding(vertical = 8.dp),
                ) {
                    Text(contact.displayName, style = MaterialTheme.typography.titleMedium)
                    contact.jobTitle?.let { Text(it, style = MaterialTheme.typography.bodyMedium) }
                }
                HorizontalDivider()
            }
        }
    }
}
