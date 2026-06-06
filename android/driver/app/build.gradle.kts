// :driver:app — Driver Android app module (home for the T14 Compose onboarding UI).
// Scaffolded by the Android bring-up with the same Compose + Paparazzi wiring as :rider:app; T14
// adds the Driver onboarding screens (incl. the Recovery-Code capture) + the ×4 a11y snapshots.
//
// Like the iOS DriverShared, the Driver REUSES the role-neutral onboarding kit — here the
// :rider:shared library (screen model/renderer/view-model/router/a11y/strings + the shared catalog) —
// and adds only the three Driver deltas (self-onboard intro, Recovery-Code capture, interactive
// re-auth phone entry). The screens RENDER the core state machine via :core-bridge (P4).

plugins {
    alias(libs.plugins.android.application)
    alias(libs.plugins.kotlin.android)
    alias(libs.plugins.kotlin.compose)
    alias(libs.plugins.paparazzi)
}

android {
    namespace = "app.boundless.driver"
    compileSdk = libs.versions.compileSdk.get().toInt()

    defaultConfig {
        applicationId = "app.boundless.driver"
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
            // CatalogDriverStrings resolves copy by parsing the REAL strings.xml files (single source
            // of truth — no English drift): the SHARED catalog in :rider:shared + the Driver's own
            // 4-key catalog. So tests + snapshots render the genuine shipped copy.
            it.systemProperty(
                "boundless.strings.path",
                rootProject.file("rider/shared/src/main/res/values/strings.xml").absolutePath,
            )
            it.systemProperty(
                "boundless.strings.driver.path",
                project.file("src/main/res/values/strings.xml").absolutePath,
            )
        }
    }
}

dependencies {
    // The shared onboarding kit (the Android twin of RiderShared); re-exports :core-bridge + Compose
    // via `api`. The Driver adds only its three deltas on top.
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
    // jnidispatch natives (the :core-bridge @aar variant is android-only). Mirrors :rider:app.
    testImplementation("net.java.dev.jna:jna:${libs.versions.jna.get()}")
}
