// The worlds (packages) the platform offers. To create a new world, make a
// folder in web/games/ with a package.js (and, if you like, translations in
// locales/) and register it here.
import terraViva from "./terra-viva/package.js";
import terraVivaEn from "./terra-viva/locales/en.js";

export const PACKAGES = [terraViva];
export const DEFAULT_PACKAGE = "terra-viva";

// Translations of each package's texts, by language. A package is written
// in one language (Terra Viva in Portuguese); these overlays replace its
// texts in the others.
export const PACKAGE_TRANSLATIONS = {
  "terra-viva": { en: terraVivaEn },
};

// Planned worlds, shown in the world setup as "coming soon". Their texts
// are in the interface's language files, under "soon.<id>".
export const COMING_SOON = ["paz-ou-guerra", "submarino"];
