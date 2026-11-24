package io.github.contactrelationshipmanagement.mobile

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Button
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.focus.focusRequester
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.unit.dp
import io.github.lilydesignsystem.compose.AlertKind
import io.github.lilydesignsystem.compose.ErrorSummary
import io.github.lilydesignsystem.compose.Field
import io.github.lilydesignsystem.compose.LilyAlert
import io.github.lilydesignsystem.compose.LilyCard
import io.github.lilydesignsystem.compose.Problem as LilyProblem
import io.github.lilydesignsystem.compose.Tag

@Composable
fun ServerScreen(model: CrmViewModel) {
    val errors = model.formErrors
    val serverFocus = remember { FocusRequester() }
    val issuerFocus = remember { FocusRequester() }
    val clientFocus = remember { FocusRequester() }
    val serverError = errors?.server?.let { stringResource(Messages.urlProblem(it)) }
    val issuerError = errors?.issuer?.let { stringResource(Messages.urlProblem(it)) }
    val clientError = if (errors?.clientIdMissing == true) stringResource(R.string.client_id_required) else null
    Column(ScreenPadding.verticalScroll(rememberScrollState()), verticalArrangement = Arrangement.spacedBy(16.dp)) {
        val problems = buildList {
            serverError?.let { add(LilyProblem(it) { serverFocus.requestFocus() }) }
            issuerError?.let { add(LilyProblem(it) { issuerFocus.requestFocus() }) }
            clientError?.let { add(LilyProblem(it) { clientFocus.requestFocus() }) }
        }
        if (problems.isNotEmpty()) ErrorSummary(stringResource(R.string.problem_title), problems)
        if (model.managed) LilyAlert(stringResource(R.string.managed_notice), kind = AlertKind.Info)
        if (model.savedNotice) LilyAlert(stringResource(R.string.saved), kind = AlertKind.Success)

        UrlField(stringResource(R.string.server_url_label), stringResource(R.string.server_url_hint), serverError, true, model.serverText, serverFocus, !model.managed) {
            model.serverText = it
        }
        UrlField(stringResource(R.string.issuer_label), stringResource(R.string.issuer_hint), issuerError, false, model.issuerText, issuerFocus, !model.managed) {
            model.issuerText = it
        }
        Field(
            label = stringResource(R.string.client_id_label),
            description = stringResource(R.string.client_id_hint),
            error = clientError,
        ) { semantics ->
            OutlinedTextField(
                value = model.clientIdText,
                onValueChange = { model.clientIdText = it },
                singleLine = true,
                enabled = !model.managed,
                isError = clientError != null,
                modifier = semantics.fillMaxWidth().focusRequester(clientFocus),
            )
        }
        if (!model.managed) Button(onClick = model::saveSettings) { Text(stringResource(R.string.save)) }

        if (model.settings?.oidc != null) {
            LilyCard(modifier = Modifier.fillMaxWidth(), heading = stringResource(R.string.sign_in)) {
                val state = stringResource(if (model.signedIn) R.string.signed_in else R.string.signed_out)
                Tag(state, label = stringResource(R.string.sign_in), live = true)
                model.signInMessage?.let { LilyAlert(text(it), kind = AlertKind.Error) }
                Row(horizontalArrangement = Arrangement.spacedBy(12.dp)) {
                    if (model.signedIn) {
                        OutlinedButton(onClick = model::signOut) { Text(stringResource(R.string.sign_out)) }
                    } else {
                        Button(onClick = model::signIn) { Text(stringResource(R.string.sign_in)) }
                    }
                }
            }
        }
    }
}

@Composable
private fun UrlField(
    label: String,
    hint: String,
    error: String?,
    required: Boolean,
    value: String,
    focus: FocusRequester,
    editable: Boolean,
    onChange: (String) -> Unit,
) {
    Field(label = label, description = hint, error = error, required = required, requiredLabel = stringResource(R.string.required)) { semantics ->
        OutlinedTextField(
            value = value,
            onValueChange = onChange,
            singleLine = true,
            enabled = editable,
            isError = error != null,
            keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Uri, autoCorrectEnabled = false),
            modifier = semantics.fillMaxWidth().focusRequester(focus),
        )
    }
}
