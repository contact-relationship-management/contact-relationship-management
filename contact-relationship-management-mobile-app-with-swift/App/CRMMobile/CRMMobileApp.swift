// The iOS app's entry point. Everything else is in the CRMMobileUI package target (../Sources).
import CRMMobileUI
import SwiftUI

@main
struct CRMMobileApp: App {
    var body: some Scene {
        WindowGroup { RootView() }
    }
}
