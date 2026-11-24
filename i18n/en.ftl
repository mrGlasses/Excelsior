# English: the fallback language (FALLBACK_LOCALE in utils/i18n.rs).
# Every key here must exist in every other .ftl file; a test checks it (I18N-3).

# Used by /hello when no ?name= is given.
world = world

hello = Hello, { $name }!

# Plural forms follow each language's CLDR rules: English has "one" and "other".
cards = You have { $count ->
    [one] { $count } card
   *[other] { $count } cards
}
