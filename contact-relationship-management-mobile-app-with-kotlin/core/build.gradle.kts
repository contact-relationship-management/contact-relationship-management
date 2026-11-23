// The app's rules and API client, in plain Kotlin on the JVM: no Android, so every rule is a JVM unit test.
import org.jetbrains.kotlin.gradle.dsl.JvmTarget

plugins {
    alias(libs.plugins.kotlin.jvm)
    alias(libs.plugins.kotlin.serialization)
}

java {
    sourceCompatibility = JavaVersion.VERSION_17
    targetCompatibility = JavaVersion.VERSION_17
}

kotlin {
    explicitApi()
    compilerOptions { jvmTarget.set(JvmTarget.JVM_17) }
}

dependencies {
    api(libs.kotlinx.serialization.json)
    api(libs.coroutines.core)
    testImplementation(kotlin("test"))
    testImplementation(libs.junit)
}
