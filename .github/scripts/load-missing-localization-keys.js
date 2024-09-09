#!/usr/bin/env node

/**
 * Helper script to load missing translation keys from the root locale file.
 * It will strip all comments and rewrite the keys to match the JSON5 standard so first changes can be large.
 * It will
 *      merge all the root locale keys in a merged object
 *      sort the keys in an alphabetic way
 *      remove the excessing keys not present in root locale
 *      overrite target locale keys with the merged keys
 * This process is not perfect but can help add missing keys in a quick way.
 *
 * Usage: node load-missing-localization-keys.js [locale to update]
 */

const path = require('path')
const fs = require('fs')
const json5 = require('json5')
// Funky console colors
STDOUT_COLORS = {
    green: '\x1b[32m',
    blue: '\x1b[34m',
    reset: '\x1b[0m'
}
console.__info = console.info
console.info = (message, ...otherArgs) => console.__info(
    STDOUT_COLORS.blue + message + STDOUT_COLORS.reset,
    ...otherArgs
)
console.success = (message, ...otherArgs) => console.info(
    STDOUT_COLORS.green + message + STDOUT_COLORS.reset,
    ...otherArgs
)

const LOCALES_PATH = path.join(__dirname, '../../vrc-get-gui/locales')
const ROOT_LOCALE = 'en'
const OTHER_LOCALES = fs.readdirSync(LOCALES_PATH)
    .map(file => file.replace(/\.\w+$/, ''))
    .filter(locale => locale !== ROOT_LOCALE)
CLI_SCRIPT_PATH = process.argv[1]
CLI_ARGUMENT_1 = process.argv[2]

// CLI Argument parsing
if (!CLI_ARGUMENT_1) {
    console.error(`Usage: node ${path.basename(CLI_SCRIPT_PATH)} [locale to update]\nAvailable locales: ${OTHER_LOCALES}`)
    process.exit(1)
}
if (!OTHER_LOCALES.includes(CLI_ARGUMENT_1)) {
    console.error(`CLI argument "${CLI_ARGUMENT_1}" is not a valid locale to update. (${OTHER_LOCALES})`)
    process.exit(1)
}

// Load all locales in a object {"en": ...keys, "fr": ...keys} 
const localesContent = [ROOT_LOCALE, CLI_ARGUMENT_1].reduce((localesObject, locale) => {
    let jsonContent = null
    try {
        const content = fs.readFileSync(path.join(LOCALES_PATH, `./${locale}.json5`))
        jsonContent = json5.parse(content)
    }
    catch (e) {
        console.error(`Could not parse json content from locale ${locale}.\n${e.stack}`)
    }
    return {
        ...localesObject,
        [locale]: jsonContent
    }
}, {})
console.info(`Loaded locales "${ROOT_LOCALE}" and "${CLI_ARGUMENT_1}" .`)
console.warn(`Note that loading the keys strips any comments present in the file.`)

console.info(`Merging root locale translations into target locale.`)
const mergedTranslationContent = {
    ...localesContent[ROOT_LOCALE].translation,
    ...localesContent[CLI_ARGUMENT_1].translation
}

const rootLocaleContentKeys = Object.keys(localesContent[ROOT_LOCALE].translation)
console.info(`Removing excessing keys not present in root  locale.`)
console.warn(`If you had upcoming keys that were not in root locale, they will be removed.`)
const cleanedTranslationContent = Object.entries(mergedTranslationContent).reduce(
    (cleanedContent, [key, value]) => {
        if (rootLocaleContentKeys.includes(key)) {
            cleanedContent[key] = value
        }
        return cleanedContent
    }, {})

console.info(`Sorting JSON keys in alphabetic order.`)
const sortedTranslationContent = Object.keys(cleanedTranslationContent).sort().reduce((obj, key) => {
    obj[key] = cleanedTranslationContent[key];
    return obj;
}, {});

console.info(`Overwriting target locale file with merged translation.`)
fs.writeFileSync(path.join(
    LOCALES_PATH, `./${CLI_ARGUMENT_1}.json5`),
    json5.stringify({
        translation: sortedTranslationContent
    }, null, 2),
    { encoding: "utf-8" }
)

console.success("Loading of missing keys done.")