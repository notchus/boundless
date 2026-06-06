package app.boundless.rider.i18n

import java.io.File
import java.util.Locale
import javax.xml.parsers.DocumentBuilderFactory
import org.w3c.dom.Element

/**
 * A test/snapshot [RiderStrings] backed by the REAL `res/values/strings.xml` — parsed directly (the
 * same file `R` is generated from), so the tests and the shipped catalog share a single source of
 * truth with no possibility of English drift. This mirrors how the iOS `RiderShared` tests resolve
 * copy via the real String Catalog (`Bundle.module`). It uses no Android `Context`, no Robolectric
 * and no emulator (pure JVM, zero new deps), which also means the Paparazzi snapshots render real
 * catalog copy WITHOUT relying on Paparazzi's own resource resolution.
 *
 * The production resolver (`AndroidRiderStrings`, over Android `Resources`) is the deferred T13-shell,
 * wired into MainActivity. Both implement only [RiderStrings.string]; the generic-fallback selection
 * and positional substitution live once in the [RiderStrings] interface.
 */
class CatalogRiderStrings private constructor(private val values: Map<String, String>) : RiderStrings {
    override fun string(key: String, vararg args: Any): String {
        val template = values[key] ?: error("Missing string resource '$key' in strings.xml")
        return if (args.isEmpty()) template else String.format(Locale.ROOT, template, *args)
    }

    companion object {
        /** Reads the catalog path from the `boundless.strings.path` system property, which the
         *  `:rider:app` build.gradle.kts `testOptions` points at the module's real strings.xml. */
        fun fromDefaultCatalog(): CatalogRiderStrings {
            val path = System.getProperty("boundless.strings.path")
                ?: error(
                    "system property 'boundless.strings.path' not set — see :rider:app build.gradle.kts testOptions",
                )
            return fromFile(File(path))
        }

        fun fromFile(file: File): CatalogRiderStrings {
            require(file.isFile) { "strings.xml not found at ${file.absolutePath}" }
            val factory = DocumentBuilderFactory.newInstance().apply {
                isNamespaceAware = false
                // Trusted local file, but disallow DTDs anyway (no XXE surface).
                runCatching { setFeature("http://apache.org/xml/features/disallow-doctype-decl", true) }
            }
            val doc = factory.newDocumentBuilder().parse(file)
            val nodes = doc.getElementsByTagName("string")
            val map = LinkedHashMap<String, String>()
            for (i in 0 until nodes.length) {
                val el = nodes.item(i) as Element
                map[el.getAttribute("name")] = unescape(el.textContent)
            }
            return CatalogRiderStrings(map)
        }

        /**
         * Apply the Android string-resource unescaping conventions (AAPT) that a JVM test must mimic
         * because it reads the raw XML, not the compiled resource: a whole-value `"…"` wrapper
         * preserves whitespace, and `\'`, `\"`, `\n`, `\t`, `\@`, `\?`, `\\` are escapes.
         */
        internal fun unescape(raw: String): String {
            var s = raw
            if (s.length >= 2 && s.startsWith("\"") && s.endsWith("\"")) s = s.substring(1, s.length - 1)
            val out = StringBuilder(s.length)
            var i = 0
            while (i < s.length) {
                val c = s[i]
                if (c == '\\' && i + 1 < s.length) {
                    when (val n = s[i + 1]) {
                        'n' -> out.append('\n')
                        't' -> out.append('\t')
                        else -> out.append(n) // \' \" \@ \? \\ → the literal next char
                    }
                    i += 2
                } else {
                    out.append(c)
                    i++
                }
            }
            return out.toString()
        }
    }
}
