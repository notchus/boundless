// :core-bridge — the Android "BoundlessCore" AAR (spec 001 Android bring-up).
//
// Packages the UniFFI binding from `core/ffi-kotlin`:
//   - src/main/kotlin/uniffi/…    generated Kotlin (git-ignored; written by build-corebridge.sh)
//   - src/main/jniLibs/<abi>/*.so cross-compiled native lib (git-ignored; cargo-ndk via that script)
// Both are reproducible build artifacts, exactly like the Swift BoundlessKit XCFramework. Run
// `scripts/build-corebridge.sh` before building/testing this module.

plugins {
    alias(libs.plugins.android.library)
    alias(libs.plugins.kotlin.android)
}

android {
    namespace = "app.boundless.corebridge"
    compileSdk = libs.versions.compileSdk.get().toInt()

    defaultConfig {
        minSdk = libs.versions.minSdk.get().toInt()
    }

    // The generated UniFFI Kotlin + the cargo-ndk .so live in the standard source sets; named
    // explicitly so the layout is unambiguous to anyone reading this module cold.
    sourceSets {
        getByName("main") {
            kotlin.srcDir("src/main/kotlin")
            jniLibs.srcDir("src/main/jniLibs")
        }
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
            // The host-JVM smoke test loads the HOST cdylib (libboundless_ffi_kotlin.{dylib,so})
            // that build-corebridge.sh leaves in core/target/release, via JNA. The on-device path
            // uses the jniLibs .so instead; this property only affects host unit tests.
            it.systemProperty(
                "jna.library.path",
                rootProject.file("../core/target/release").absolutePath,
            )
        }
    }
}

dependencies {
    // Android runtime: the @aar variant bundles JNA's own per-ABI native dispatch lib into the APK.
    implementation("net.java.dev.jna:jna:${libs.versions.jna.get()}@aar")
    // Host JVM unit test: the plain jar bundles the host (macOS/Linux) jnidispatch natives so the
    // generated bindings can JNA-load the host cdylib above without an emulator.
    testImplementation("net.java.dev.jna:jna:${libs.versions.jna.get()}")
    testImplementation(libs.junit)
}
