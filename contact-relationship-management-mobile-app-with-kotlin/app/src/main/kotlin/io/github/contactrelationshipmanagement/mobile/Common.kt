package io.github.contactrelationshipmanagement.mobile

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material3.Button
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.node.ModifierNodeElement
import androidx.compose.ui.platform.PlatformTextInputInterceptor
import androidx.compose.ui.platform.PlatformTextInputMethodRequest
import androidx.compose.ui.platform.PlatformTextInputModifierNode
import androidx.compose.ui.platform.PlatformTextInputSession
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.dp
import io.github.contactrelationshipmanagement.mobile.core.Money
import io.github.contactrelationshipmanagement.mobile.core.MoneyText
import android.view.inputmethod.EditorInfo
import io.github.lilydesignsystem.compose.AlertKind
import io.github.lilydesignsystem.compose.LilyAlert
import java.util.Locale

@Composable
fun text(message: Message): String {
    val context = LocalContext.current
    return context.getString(message.id, *message.args.toTypedArray())
}

/** Shows [load]: a spinner, the error with a retry, or [content]. Loads once when first shown. */
@Composable
fun <T> Loading(load: Load<T>, reload: () -> Unit, content: @Composable (T) -> Unit) {
    LaunchedEffect(Unit) { if (load is Load.Idle) reload() }
    when (load) {
        Load.Idle, Load.Loading -> {
            val label = stringResource(R.string.loading)
            CircularProgressIndicator(Modifier.padding(24.dp).semantics { contentDescription = label })
        }
        is Load.Failed -> Column(Modifier.padding(16.dp), verticalArrangement = Arrangement.spacedBy(12.dp)) {
            LilyAlert(text(load.message), kind = AlertKind.Error)
            Button(onClick = reload) { Text(stringResource(R.string.retry)) }
        }
        is Load.Loaded -> content(load.value)
    }
}

/** An amount as MOB-6 says: exact, "Hidden" when masked, and an error, not zero, when unreadable. */
@Composable
fun amount(minorText: String?, currency: String): String {
    val locale: Locale = LocalContext.current.resources.configuration.locales[0]
    return when (val money = Money.format(minorText, currency, locale)) {
        MoneyText.Hidden -> stringResource(R.string.amount_hidden)
        is MoneyText.Malformed -> stringResource(R.string.amount_malformed)
        is MoneyText.Shown -> if (money.knownCurrency) money.text else stringResource(R.string.amount_unknown_currency, money.text)
    }
}

val ScreenPadding = Modifier.fillMaxWidth().padding(16.dp)

/**
 * MOB-15: what the rep types is not kept in the keyboard's dictionary. Compose has no option for
 * `IME_FLAG_NO_PERSONALIZED_LEARNING`, so it is added to the editor info the keyboard is given.
 */
fun Modifier.noKeyboardLearning(): Modifier = this then NoKeyboardLearningElement

private object NoKeyboardLearningElement : ModifierNodeElement<NoKeyboardLearningNode>() {
    override fun create() = NoKeyboardLearningNode()
    override fun update(node: NoKeyboardLearningNode) = Unit
    override fun hashCode() = 1
    override fun equals(other: Any?) = other === this
}

private class NoKeyboardLearningNode : Modifier.Node(), PlatformTextInputModifierNode, PlatformTextInputInterceptor {
    override suspend fun interceptStartInputMethod(request: PlatformTextInputMethodRequest, nextHandler: PlatformTextInputSession): Nothing =
        nextHandler.startInputMethod(
            PlatformTextInputMethodRequest { outAttrs ->
                request.createInputConnection(outAttrs).also {
                    outAttrs.imeOptions = outAttrs.imeOptions or EditorInfo.IME_FLAG_NO_PERSONALIZED_LEARNING
                }
            },
        )
}

/** Autocorrect off in the same field, so a typed correction is not learned either. */
val NoAutocorrect = KeyboardOptions(autoCorrectEnabled = false)
