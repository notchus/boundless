// :rider:shared — the role-neutral Boundless onboarding kit (spec 001 T14 extraction).
//
// The Android twin of the iOS `RiderShared` library: the screen model + the single Compose renderer
// (`OnboardingScreenView`), the onboarding view model + router, the a11y descriptors, the role-neutral
// screen factories (`RiderOnboardingScreens`), the `RiderStrings` catalog interface, and the shared
// `res/values/strings.xml`. The screens RENDER the `core::auth` state machine (exported by
// `:core-bridge`) — they never decide transitions (P4). Both `:rider:app` (T13) and `:driver:app`
// (T14) depend on this; the Driver adds only its three deltas + a 4-key catalog. An app module can't
// depend on another app module, so the shared kit is a com.android.library.
//
// The catalog-parse test/snapshot resolver (`CatalogRiderStrings`) lives in this module's `main`
// source set so BOTH apps' test source sets can consume it single-sourced. (AGP testFixtures would be
// the natural home, but KGP 2.0.21 — pinned with AGP 8.4.2 / Paparazzi 1.3.5 — doesn't compile Kotlin
// in `testFixtures` source sets; that support landed in Kotlin 2.1.0. It is inert in production: the
// shipping resolver is the Resources-backed `AndroidRiderStrings`, the deferred app shell.)

plugins {
    alias(libs.plugins.android.library)
    alias(libs.plugins.kotlin.android)
    alias(libs.plugins.kotlin.compose)
}

android {
    namespace = "app.boundless.rider.shared"
    compileSdk = libs.versions.compileSdk.get().toInt()

    defaultConfig {
        minSdk = libs.versions.minSdk.get().toInt()
    }

    buildFeatures {
        compose = true
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }
    kotlinOptions {
        jvmTarget = "17"
    }
}

dependencies {
    // `api` for what appears in this module's PUBLIC surface: the UniFFI types (the view model's
    // ctor takes `Role`; the router switches on `OnboardingState`) and the Compose APIs (the public
    // `@Composable` renderer). Consumers (the apps) thus see them transitively, matching iOS where
    // DriverShared sees BoundlessKit + SwiftUI through RiderShared.
    api(project(":core-bridge"))
    api(libs.androidx.compose.ui)
    api(libs.androidx.compose.foundation)
    api(libs.androidx.compose.material3)
    implementation(libs.androidx.compose.ui.tooling.preview)
    // The router uses rememberCoroutineScope + the view model's suspend boundaries.
    implementation(libs.kotlinx.coroutines.android)
}
