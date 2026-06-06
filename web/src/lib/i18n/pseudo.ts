// Pseudo-locale (`zz-ZZ`) generation for AC12 (P8) — spec 001 T16.
//
// The pseudo-locale is the deterministic accent+pad+bracket transform of the source catalog.
// It is the i18n discipline tool the constitution/voice-and-tone call for ("Pseudo-locale
// (zz-ZZ) builds catch hardcoded strings. Every screen must render in pseudo-locale without
// breaking."): a screen rendered in `zz-ZZ` makes two classes of bug *visible* —
//
//   1. a hardcoded string literal shows up un-bracketed (it didn't go through the catalog), and
//   2. a too-tight layout truncates the padded text (the ~40% expansion surfaces it).
//
// It is generated, not translated — real `gsw`/RTL/`zz-ZZ` translations arrive via the Weblate
// + signed-KV pipeline (ADR-0014). `catalog.ts` wires `catalogs['zz-ZZ'] = pseudoCatalog(en)`;
// the default locale stays `en`, so production copy is unaffected (zz-ZZ is opt-in via
// `?locale=zz-ZZ`, used by the AC12 render test and by hand for QA).

const ACCENTS: Record<string, string> = {
	a: 'á', b: 'ƀ', c: 'ç', d: 'ð', e: 'é', f: 'ƒ', g: 'ĝ', h: 'ĥ', i: 'í', j: 'ĵ',
	k: 'ķ', l: 'ļ', m: 'ɱ', n: 'ñ', o: 'ó', p: 'þ', q: 'ɋ', r: 'ŕ', s: 'š', t: 'ţ',
	u: 'ú', v: 'ṽ', w: 'ŵ', x: 'ẋ', y: 'ý', z: 'ž',
	A: 'Á', B: 'Ɓ', C: 'Ç', D: 'Ð', E: 'É', F: 'Ƒ', G: 'Ĝ', H: 'Ĥ', I: 'Í', J: 'Ĵ',
	K: 'Ķ', L: 'Ļ', M: 'Ṁ', N: 'Ñ', O: 'Ó', P: 'Þ', Q: 'Ɋ', R: 'Ŕ', S: 'Š', T: 'Ţ',
	U: 'Ú', V: 'Ṽ', W: 'Ŵ', X: 'Ẋ', Y: 'Ý', Z: 'Ž',
};

const FILLER = 'áéíóúàèìòù';

/** Opening/closing markers; a rendered screen showing these proves the string came from the
 *  catalog (and the QA eye can spot any un-bracketed literal). */
export const PSEUDO_OPEN = '⟦';
export const PSEUDO_CLOSE = '⟧';

/**
 * Pseudo-localize one message. Accents ASCII letters, pads ~40% (to surface truncation), and
 * brackets the whole. **ICU placeholders (`{argName}`, incl. nested plural/select braces) and
 * `<tags>` are copied verbatim** so the message still parses + interpolates identically.
 */
export function pseudoize(msg: string): string {
	let out = '';
	let letters = 0;
	let i = 0;
	while (i < msg.length) {
		const ch = msg[i] as string;
		if (ch === '{') {
			// Copy a balanced {...} ICU argument verbatim (handles nested plural/select braces).
			let depth = 0;
			const start = i;
			while (i < msg.length) {
				if (msg[i] === '{') depth++;
				else if (msg[i] === '}') {
					depth--;
					if (depth === 0) {
						i++;
						break;
					}
				}
				i++;
			}
			out += msg.slice(start, i);
			continue;
		}
		if (ch === '<') {
			// Copy a <tag> verbatim.
			const start = i;
			while (i < msg.length && msg[i] !== '>') i++;
			if (i < msg.length) i++; // include '>'
			out += msg.slice(start, i);
			continue;
		}
		out += ACCENTS[ch] ?? ch;
		if (/[A-Za-z]/.test(ch)) letters++;
		i++;
	}
	const padCount = Math.max(2, Math.ceil(letters * 0.4));
	let pad = '';
	for (let k = 0; k < padCount; k++) pad += FILLER[k % FILLER.length];
	return `${PSEUDO_OPEN}${out} ${pad}${PSEUDO_CLOSE}`;
}

/** Pseudo-localize every value of a catalog (keys unchanged). */
export function pseudoCatalog(source: Record<string, string>): Record<string, string> {
	const out: Record<string, string> = {};
	for (const [key, value] of Object.entries(source)) out[key] = pseudoize(value);
	return out;
}
