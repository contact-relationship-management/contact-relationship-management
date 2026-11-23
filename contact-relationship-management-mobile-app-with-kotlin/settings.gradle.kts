// Contact Relationship Management for Android (spec/mobile-apps, CRM-T303).
// Lily Design System for Kotlin is not published yet, so it is built from the commit pinned in
// ../lily-design-system.lock, which scripts/fetch-lily.sh fetches into ../vendor (CRM-D28).
pluginManagement {
    repositories {
        google()
        mavenCentral()
        gradlePluginPortal()
    }
}
dependencyResolutionManagement {
    repositoriesMode.set(RepositoriesMode.FAIL_ON_PROJECT_REPOS)
    repositories {
        google()
        mavenCentral()
    }
}

val lily = file("../vendor/lily-design-system/lily-design-system-kotlin")
require(lily.resolve("settings.gradle.kts").isFile) {
    "Lily Design System for Kotlin is missing at $lily. Run scripts/fetch-lily.sh from the repository root first."
}
includeBuild(lily) {
    dependencySubstitution {
        substitute(module("io.github.lilydesignsystem:lily-compose")).using(project(":lily-compose"))
        substitute(module("io.github.lilydesignsystem:lily-compose-themes")).using(project(":lily-compose-themes"))
    }
}

rootProject.name = "contact-relationship-management-mobile-app-with-kotlin"
include(":core", ":app")
