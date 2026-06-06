// Boundless Android — Gradle settings (spec 001 Android bring-up).
//
// The Android side of the monorepo. The native domain/auth logic is NOT reimplemented here:
// it is the Rust core, surfaced across UniFFI by `core/ffi-kotlin` and packaged into the
// `:core-bridge` module's AAR (the P4 "BoundlessCore"). The Compose UIs (`:rider:app` T13,
// `:driver:app` T14) render the core state machine — they hold no hand-rolled auth logic.
//
// The role-neutral onboarding kit (screen model/renderer/view-model/router/a11y/strings + the
// shared string catalog) lives in `:rider:shared` (a com.android.library), the Android twin of the
// iOS `RiderShared` library. Both apps depend on it; `:driver:app` adds only the Driver deltas
// (T14). An app module can't depend on another app module, so the shared kit must be a library.

pluginManagement {
    repositories {
        google {
            content {
                includeGroupByRegex("com\\.android.*")
                includeGroupByRegex("com\\.google.*")
                includeGroupByRegex("androidx.*")
            }
        }
        mavenCentral()
        gradlePluginPortal()
    }
}

@Suppress("UnstableApiUsage")
dependencyResolutionManagement {
    repositoriesMode.set(RepositoriesMode.FAIL_ON_PROJECT_REPOS)
    repositories {
        google()
        mavenCentral()
    }
}

rootProject.name = "boundless-android"

include(":core-bridge")
include(":rider:shared")
include(":rider:app")
include(":driver:app")
