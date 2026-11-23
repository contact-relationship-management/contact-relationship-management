// MOB-6: an amount is exact, and a masked amount is not zero.
import Foundation

public enum MoneyError: Error, Equatable, Sendable {
    /// The amount text was not an optional `-` followed by digits.
    case notAnInteger(String)
}

public enum Money {
    /// How many minor digits `currency` has; two when the platform does not know the code.
    public static func minorDigits(_ currency: String) -> Int {
        let code = currency.uppercased()
        guard Locale.commonISOCurrencyCodes.contains(code) else { return 2 }
        let formatter = NumberFormatter()
        formatter.numberStyle = .currency
        formatter.currencyCode = code
        return formatter.maximumFractionDigits
    }

    /// The amount `minorText` minor units of `currency`, exactly, as a `Decimal`.
    public static func decimal(minorText: String, currency: String) throws(MoneyError) -> Decimal {
        let digits = minorText.hasPrefix("-") ? minorText.dropFirst() : Substring(minorText)
        guard !digits.isEmpty, digits.allSatisfy({ $0.isASCII && $0.isNumber }),
            let minor = Decimal(string: minorText, locale: Locale(identifier: "en_US_POSIX"))
        else { throw .notAnInteger(minorText) }
        var scaled = Decimal()
        var value = minor
        _ = NSDecimalMultiplyByPowerOf10(&scaled, &value, Int16(-minorDigits(currency)), .plain)
        return scaled
    }

    /// The amount shown to a reader in `locale`; `nil` when the amount is masked (`minorText` is `nil`).
    public static func format(minorText: String?, currency: String, locale: Locale) throws(MoneyError) -> String? {
        guard let minorText else { return nil }
        let amount = try decimal(minorText: minorText, currency: currency)
        let code = currency.uppercased()
        let digits = minorDigits(code)
        let formatter = NumberFormatter()
        formatter.locale = locale
        formatter.generatesDecimalNumbers = true
        if Locale.commonISOCurrencyCodes.contains(code) {
            formatter.numberStyle = .currency
            formatter.currencyCode = code
            formatter.minimumFractionDigits = digits
            formatter.maximumFractionDigits = digits
            return formatter.string(from: amount as NSDecimalNumber)
        }
        formatter.numberStyle = .decimal
        formatter.minimumFractionDigits = digits
        formatter.maximumFractionDigits = digits
        return formatter.string(from: amount as NSDecimalNumber).map { "\($0) \(code)" }
    }
}
