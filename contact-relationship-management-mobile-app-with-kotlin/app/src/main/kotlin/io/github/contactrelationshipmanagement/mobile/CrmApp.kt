package io.github.contactrelationshipmanagement.mobile

import androidx.activity.compose.BackHandler
import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.material3.VerticalDivider
import androidx.compose.ui.platform.LocalConfiguration
import androidx.compose.ui.unit.dp
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.NavigationBar
import androidx.compose.material3.NavigationBarItem
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
import androidx.compose.material3.darkColorScheme
import androidx.compose.material3.lightColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.material3.Surface
import io.github.lilydesignsystem.compose.themes.LilyTheme
import io.github.lilydesignsystem.compose.themes.LilyTokens

/** The width, in dp, from which the list and the record sit side by side (Material's medium window class and up, MOB-13). */
private const val WIDE_DP = 600

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun CrmApp(model: CrmViewModel) {
    val dark = isSystemInDarkTheme()
    MaterialTheme(colorScheme = if (dark) darkColorScheme() else lightColorScheme()) {
        LilyTheme(if (dark) LilyTokens.dark else LilyTokens.light) {
            BackHandler(enabled = model.screen !is Screen.Home) { model.back() }
            val screen = model.screen
            val back = stringResource(R.string.back)
            Scaffold(
                topBar = {
                    TopAppBar(
                        title = { Text(title(screen)) },
                        navigationIcon = {
                            if (screen !is Screen.Home) {
                                IconButton(onClick = { model.back() }) {
                                    Text("‹", style = MaterialTheme.typography.headlineSmall, modifier = Modifier.semantics { contentDescription = back })
                                }
                            }
                        },
                    )
                },
                bottomBar = {
                    NavigationBar {
                        val current = (screen as? Screen.Home)?.tab ?: Tab.Contacts
                        for (tab in Tab.entries) {
                            NavigationBarItem(
                                selected = current == tab,
                                onClick = { model.select(tab) },
                                icon = {},
                                label = { Text(stringResource(tabLabel(tab))) },
                            )
                        }
                    }
                },
            ) { padding ->
                Surface(Modifier.padding(padding)) {
                    Box {
                        // MOB-13: on a wide screen the contact list stays beside the contact it opened.
                        val wide = LocalConfiguration.current.screenWidthDp >= WIDE_DP
                        when (screen) {
                            is Screen.Home -> when (screen.tab) {
                                Tab.Contacts -> if (wide) {
                                    ListAndRecord(model) { Text(stringResource(R.string.contacts_select), Modifier.padding(16.dp)) }
                                } else {
                                    ContactsScreen(model)
                                }
                                Tab.Deals -> DealsScreen(model)
                                Tab.Tickets -> TicketsScreen(model)
                                Tab.Server -> ServerScreen(model)
                            }
                            is Screen.ContactScreen -> if (wide) {
                                ListAndRecord(model) { ContactScreen(model, screen.pid) }
                            } else {
                                ContactScreen(model, screen.pid)
                            }
                            is Screen.LogActivity -> LogActivityScreen(model, screen.pid)
                        }
                    }
                }
            }
        }
    }
}

/** The contact list with [record] beside it (MOB-13). */
@Composable
private fun ListAndRecord(model: CrmViewModel, record: @Composable () -> Unit) {
    Row(Modifier.fillMaxSize()) {
        Box(Modifier.weight(1f)) { ContactsScreen(model) }
        VerticalDivider()
        Box(Modifier.weight(2f)) { record() }
    }
}

@Composable
private fun title(screen: Screen): String = when (screen) {
    is Screen.Home -> stringResource(tabLabel(screen.tab))
    is Screen.ContactScreen -> stringResource(R.string.contact_details)
    is Screen.LogActivity -> stringResource(R.string.log_activity)
}

fun tabLabel(tab: Tab): Int = when (tab) {
    Tab.Contacts -> R.string.tab_contacts
    Tab.Deals -> R.string.tab_deals
    Tab.Tickets -> R.string.tab_tickets
    Tab.Server -> R.string.tab_server
}
