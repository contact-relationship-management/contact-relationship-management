package io.github.contactrelationshipmanagement.mobile

import android.app.Application
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.lifecycle.AndroidViewModel
import androidx.lifecycle.viewModelScope
import io.github.contactrelationshipmanagement.mobile.core.ACTIVITY_KINDS
import io.github.contactrelationshipmanagement.mobile.core.ActivityPayload
import io.github.contactrelationshipmanagement.mobile.core.ApiClient
import io.github.contactrelationshipmanagement.mobile.core.ApiException
import io.github.contactrelationshipmanagement.mobile.core.Contact
import io.github.contactrelationshipmanagement.mobile.core.ContactDetail
import io.github.contactrelationshipmanagement.mobile.core.DealBoard
import io.github.contactrelationshipmanagement.mobile.core.IdempotencyKey
import io.github.contactrelationshipmanagement.mobile.core.Listed
import io.github.contactrelationshipmanagement.mobile.core.OidcClient
import io.github.contactrelationshipmanagement.mobile.core.Session
import io.github.contactrelationshipmanagement.mobile.core.Settings
import io.github.contactrelationshipmanagement.mobile.core.SettingsForm
import io.github.contactrelationshipmanagement.mobile.core.Ticket
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.channels.Channel
import kotlinx.coroutines.flow.receiveAsFlow
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext

/** A screen's data: in memory only (MOB-1). */
sealed interface Load<out T> {
    data object Idle : Load<Nothing>
    data object Loading : Load<Nothing>
    data class Loaded<T>(val value: T) : Load<T>
    data class Failed(val message: Message) : Load<Nothing>
}

enum class Tab { Contacts, Deals, Tickets, Server }

sealed interface Screen {
    data class Home(val tab: Tab) : Screen
    data class ContactScreen(val pid: String) : Screen
    data class LogActivity(val pid: String) : Screen
}

/** The log-activity form. [key] is kept across retries until the activity is saved (MOB-7). */
data class LogForm(
    val kind: String = ACTIVITY_KINDS.first(),
    val summary: String = "",
    val summaryMissing: Boolean = false,
    val key: IdempotencyKey = IdempotencyKey(),
    val sending: Boolean = false,
    val sent: Boolean = false,
    val error: Message? = null,
)

class CrmViewModel(application: Application) : AndroidViewModel(application) {
    private val store = SettingsStore(application)
    private val session = Session()

    var serverText by mutableStateOf("")
    var issuerText by mutableStateOf("")
    var clientIdText by mutableStateOf("")
    var formErrors by mutableStateOf<SettingsForm.Errors?>(null)
        private set
    var savedNotice by mutableStateOf(false)
        private set
    var settings by mutableStateOf<Settings?>(null)
        private set
    var signedIn by mutableStateOf(false)
        private set
    var signInMessage by mutableStateOf<Message?>(null)
        private set

    var screen by mutableStateOf<Screen>(Screen.Home(Tab.Contacts))
        private set
    var contacts by mutableStateOf<Load<Listed<Contact>>>(Load.Idle)
        private set
    var filter by mutableStateOf("")
    var detail by mutableStateOf<Load<ContactDetail>>(Load.Idle)
        private set
    var deals by mutableStateOf<Load<DealBoard.Grouped>>(Load.Idle)
        private set
    var tickets by mutableStateOf<Load<List<Ticket>>>(Load.Idle)
        private set
    var logForm by mutableStateOf(LogForm())
        private set

    private val browser = Channel<String>(Channel.BUFFERED)

    /** URLs to open in the system browser (MOB-4). */
    val browserRequests = browser.receiveAsFlow()

    init {
        val saved = store.load()
        serverText = saved.server
        issuerText = saved.issuer
        clientIdText = saved.clientId
        settings = (SettingsForm.check(saved.server, saved.issuer, saved.clientId) as? SettingsForm.Result.Valid)?.settings
        if (settings == null) screen = Screen.Home(Tab.Server)
    }

    private fun client(): ApiClient? = settings?.let { s -> ApiClient(s.server) { session.accessToken } }

    fun saveSettings() {
        savedNotice = false
        when (val result = SettingsForm.check(serverText, issuerText, clientIdText)) {
            is SettingsForm.Result.Invalid -> formErrors = result.errors
            is SettingsForm.Result.Valid -> {
                formErrors = null
                store.save(SettingsStore.Saved(serverText, issuerText, clientIdText))
                if (result.settings != settings) {
                    signOut()
                    settings = result.settings
                    contacts = Load.Idle
                    deals = Load.Idle
                    tickets = Load.Idle
                }
                savedNotice = true
            }
        }
    }

    fun select(tab: Tab) {
        screen = Screen.Home(tab)
        when (tab) {
            Tab.Contacts -> if (contacts !is Load.Loaded) loadContacts()
            Tab.Deals -> if (deals !is Load.Loaded) loadDeals()
            Tab.Tickets -> if (tickets !is Load.Loaded) loadTickets()
            Tab.Server -> Unit
        }
    }

    fun openContact(pid: String) {
        screen = Screen.ContactScreen(pid)
        loadDetail(pid)
    }

    fun openLog(pid: String) {
        logForm = LogForm()
        screen = Screen.LogActivity(pid)
    }

    /** Back: from a form to its contact, from a contact to the list. False when there is nowhere to go back to. */
    fun back(): Boolean = when (val current = screen) {
        is Screen.LogActivity -> { openContact(current.pid); true }
        is Screen.ContactScreen -> { screen = Screen.Home(Tab.Contacts); true }
        is Screen.Home -> false
    }

    fun loadContacts() = load({ contacts = it }) { it.contacts() }

    fun loadDetail(pid: String) = load({ detail = it }) { it.contact(pid) }

    fun loadDeals() = load({ deals = it }) { DealBoard.group(it.pipelines(), it.deals()) }

    fun loadTickets() = load({ tickets = it }) { it.tickets() }

    private fun <T> load(set: (Load<T>) -> Unit, fetch: (ApiClient) -> T) {
        val api = client() ?: return set(Load.Failed(Message(R.string.no_server)))
        set(Load.Loading)
        viewModelScope.launch {
            set(
                try {
                    Load.Loaded(withContext(Dispatchers.IO) { fetch(api) })
                } catch (e: ApiException) {
                    if (e is ApiException.Unauthorized) signOut()
                    Load.Failed(Messages.error(e, settings?.oidc != null))
                },
            )
        }
    }

    fun setLogKind(kind: String) {
        logForm = logForm.copy(kind = kind, sent = false)
    }

    fun setLogSummary(summary: String) {
        logForm = logForm.copy(summary = summary, summaryMissing = false, sent = false)
    }

    /** Sends the form; a retry after a failure reuses the form's key (MOB-7). */
    fun sendLog(pid: String) {
        val form = logForm
        if (form.sending) return
        if (form.summary.isBlank()) {
            logForm = form.copy(summaryMissing = true)
            return
        }
        val api = client() ?: return
        logForm = form.copy(sending = true, error = null)
        viewModelScope.launch {
            logForm = try {
                withContext(Dispatchers.IO) { api.logActivity(ActivityPayload(form.kind, form.summary, "contact", pid), form.key) }
                loadDetail(pid)
                LogForm(kind = form.kind, sent = true)
            } catch (e: ApiException) {
                if (e is ApiException.Unauthorized) signOut()
                logForm.copy(sending = false, error = Messages.error(e, settings?.oidc != null))
            }
        }
    }

    fun signIn() {
        val oidc = settings?.oidc ?: run { signInMessage = Message(R.string.sign_in_not_configured); return }
        signInMessage = null
        viewModelScope.launch {
            try {
                val endpoints = withContext(Dispatchers.IO) { OidcClient(oidc).discover() }
                browser.send(session.begin(endpoints, oidc.clientId))
            } catch (e: ApiException) {
                signInMessage = Messages.error(e)
            }
        }
    }

    /** The browser came back to the redirect URI (MOB-4). */
    fun completeSignIn(uri: String) {
        val oidc = settings?.oidc ?: return
        when (val completion = session.complete(uri)) {
            is Session.Completion.Refused -> signInMessage = Messages.callback(completion.error)
            is Session.Completion.Exchange -> viewModelScope.launch {
                try {
                    val token = withContext(Dispatchers.IO) {
                        OidcClient(oidc).exchange(completion.endpoints, completion.code, completion.verifier)
                    }
                    session.signedIn(token.accessToken)
                    signedIn = true
                    signInMessage = null
                    contacts = Load.Idle
                    deals = Load.Idle
                    tickets = Load.Idle
                    select(Tab.Contacts)
                } catch (e: ApiException) {
                    signInMessage = Messages.error(e)
                }
            }
        }
    }

    fun noBrowser() {
        signInMessage = Message(R.string.sign_in_no_browser)
    }

    fun signOut() {
        session.signOut()
        signedIn = false
    }
}
