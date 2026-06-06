// :rider:app — Rider Android app module (home for the T13 Compose onboarding UI).
// Scaffolded by the Android bring-up: Compose + Paparazzi are wired and proven green by a sample
// snapshot; T13 adds the real onboarding screens (rendered from :core-bridge) + the ×4 a11y
// snapshot matrix (default / largest font / dark / RTL) the a11y bar requires.

plugins {
    alias(libs.plugins.android.application)
    alias(libs.plugins.kotlin.android)
    alias(libs.plugins.kotlin.compose)
    alias(libs.plugins.paparazzi)
}

android {
    namespace = "app.boundless.rider"
    compileSdk = libs.versions.compileSdk.get().toInt()

    defaultConfig {
        applicationId = "app.boundless.rider"
        minSdk = libs.versions.minSdk.get().toInt()
        targetSdk = libs.versions.targetSdk.get().toInt()
        versionCode = 1
        versionName = "0.1.0"
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

    testOptions {
        unitTests.all {
            // The onboarding logic tests drive the core state machine through the :core-bridge UniFFI
            // bindings, which JNA-load the HOST cdylib that build-corebridge.sh leaves in
            // core/target/release (no emulator). The Paparazzi snapshots don't touch the FFI.
            it.systemProperty(
                "jna.library.path",
                rootProject.file("../core/target/release").absolutePath,
            )
            // CatalogRiderStrings resolves copy by parsing the REAL strings.xml (single source of
            // truth — no English drift), so tests + snapshots render the genuine shipped catalog.
            // The shared catalog now lives in :rider:shared (merged into the app at build time).
            it.systemProperty(
                "boundless.strings.path",
                rootProject.file("rider/shared/src/main/res/values/strings.xml").absolutePath,
            )
        }
    }
}

dependencies {
    // The role-neutral onboarding kit (screens/renderer/view-model/router/a11y/strings + the shared
    // catalog) lives in :rider:shared; the app keeps only RiderSettings + RiderTheme. :rider:shared
    // re-exports :core-bridge + Compose via `api`, but the explicit deps below keep the app's own
    // surface self-documenting (and the FFI-driving tests import uniffi types directly).
    implementation(project(":rider:shared"))
    implementation(project(":core-bridge"))
    implementation(libs.androidx.compose.ui)
    implementation(libs.androidx.compose.foundation)
    implementation(libs.androidx.compose.material3)
    implementation(libs.androidx.compose.ui.tooling.preview)
    implementation(libs.kotlinx.coroutines.android)

    testImplementation(libs.junit)
    testImplementation(libs.kotlinx.coroutines.test)
    // Host JVM unit tests load the UniFFI bindings via JNA; the plain jar bundles the host
    // jnidispatch natives (the :core-bridge @aar variant is android-only). Mirrors :core-bridge.
    testImplementation("net.java.dev.jna:jna:${libs.versions.jna.get()}")
}
