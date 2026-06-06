// Boundless Android — root build (spec 001 Android bring-up).
// Plugins are declared here (apply false) and applied per-module, so versions are single-sourced
// from gradle/libs.versions.toml.

plugins {
    alias(libs.plugins.android.application) apply false
    alias(libs.plugins.android.library) apply false
    alias(libs.plugins.kotlin.android) apply false
    alias(libs.plugins.kotlin.compose) apply false
    alias(libs.plugins.paparazzi) apply false
}
