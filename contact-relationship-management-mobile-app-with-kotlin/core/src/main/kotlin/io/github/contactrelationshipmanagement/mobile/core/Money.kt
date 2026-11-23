package io.github.contactrelationshipmanagement.mobile.core

import java.math.BigDecimal
import java.text.DecimalFormat
import java.text.NumberFormat
import java.util.Currency
import java.util.Locale

/** An amount as shown (MOB-6). */
public sealed interface MoneyText {
    /** The amount is masked for this reader: shown as "Hidden", never as zero. */
    public data object Hidden : MoneyText

    /** The amount, formatted; [knownCurrency] is false when the platform does not know the code. */
    public data class Shown(val text: String, val knownCurrency: Boolean) : MoneyText

    /** The service sent text that is not an integer: an error, not zero. */
    public data class Malformed(val input: String) : MoneyText
}

public object Money {
    private val INTEGER = Regex("-?[0-9]+")

    /** Minor-unit digits assumed for a code the platform does not know. */
    public const val UNKNOWN_CURRENCY_DIGITS: Int = 2

    /** Formats `amount_minor_text` in [currency] for [locale], in decimal arithmetic, never through a float. */
    public fun format(amountMinorText: String?, currency: String, locale: Locale): MoneyText {
        if (amountMinorText == null) return MoneyText.Hidden
        if (!INTEGER.matches(amountMinorText)) return MoneyText.Malformed(amountMinorText)
        val minor = BigDecimal(amountMinorText)
        val known = knownCurrency(currency)
        return if (known != null) {
            val digits = known.defaultFractionDigits
            val format = NumberFormat.getCurrencyInstance(locale).apply {
                this.currency = known
                minimumFractionDigits = digits
                maximumFractionDigits = digits
            }
            MoneyText.Shown(format.format(minor.movePointLeft(digits)), knownCurrency = true)
        } else {
            val format = (NumberFormat.getNumberInstance(locale) as DecimalFormat).apply {
                minimumFractionDigits = UNKNOWN_CURRENCY_DIGITS
                maximumFractionDigits = UNKNOWN_CURRENCY_DIGITS
            }
            MoneyText.Shown("${format.format(minor.movePointLeft(UNKNOWN_CURRENCY_DIGITS))} $currency", knownCurrency = false)
        }
    }

    /** The platform's currency for [code], or null when it has none or no fixed minor-unit digits (a fund, gold). */
    private fun knownCurrency(code: String): Currency? {
        val currency = try {
            Currency.getInstance(code)
        } catch (_: IllegalArgumentException) {
            return null
        }
        return currency.takeIf { it.defaultFractionDigits >= 0 }
    }
}
