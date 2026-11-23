package io.github.contactrelationshipmanagement.mobile.core

import java.util.Locale
import kotlin.test.Test
import kotlin.test.assertEquals

class MoneyTest {
    private fun shown(text: String?, currency: String): MoneyText.Shown = Money.format(text, currency, Locale.US) as MoneyText.Shown

    @Test fun maskedIsHiddenNotZero() = assertEquals(MoneyText.Hidden, Money.format(null, "GBP", Locale.US))

    @Test fun twoMinorDigits() {
        assertEquals("£1,234.56", shown("123456", "GBP").text)
        assertEquals("$0.05", shown("5", "USD").text)
        assertEquals("-$12.30", shown("-1230", "USD").text)
    }

    @Test fun zeroAndThreeMinorDigits() {
        assertEquals("¥1,234", shown("1234", "JPY").text)
        assertEquals("KWD1.234", shown("1234", "KWD").text)
    }

    @Test fun beyondLongAndDoubleIsExact() {
        // 2^63 + 1 cents: more than a Long holds, and a double would round it.
        assertEquals("$92,233,720,368,547,758.09", shown("9223372036854775809", "USD").text)
    }

    @Test fun anUnknownCodeGetsTwoDigitsAndItsCode() {
        val money = shown("123456", "ZZZ")
        assertEquals("1,234.56 ZZZ", money.text)
        assertEquals(false, money.knownCurrency)
        assertEquals(true, shown("1", "EUR").knownCurrency)
    }

    @Test fun malformedTextIsAnErrorNotZero() {
        for (bad in listOf("", "-", "1.5", "1e3", " 12", "12 ", "+12", "--1", "١٢")) {
            assertEquals(MoneyText.Malformed(bad), Money.format(bad, "USD", Locale.US), bad)
        }
    }

    @Test fun theExactTextIsPreferredToTheNumber() {
        val deal = Deal("p", "n", "USD", amountMinor = 1, amountMinorText = "2", pipelinePid = "a", stagePid = "b", kanbanPosition = 0, won = false)
        assertEquals("2", deal.exactMinorText)
        assertEquals("1", deal.copy(amountMinorText = null).exactMinorText)
        assertEquals(null, deal.copy(amountMinor = null, amountMinorText = null).exactMinorText)
    }
}
