# Contributing

## Localizing

This project is internationalized, so when you add some text contents to the project, 
please add or use an existing localization key and i18n instead of hardcoding the text.

When you add a new localization key, you have to add value for english (`locales/en.json5`).
If you understand other languages, you can add values for them but if you don't, please don't add them.
Maintainers of each language will add them.

## Adding languages

It's welcome to add new languages to the project.
If you want to add a new language, it will follow the following steps.

1. You fork the repository and create branch for the new language.
2. You create a new json5 file in `locales` folder with the language code.
   - For example, if you want to add Japanese, create `ja.json5`.
3. You edit code to import the new json5 file in `lib/i18n.ts` and add it to the `languageResources` object.
4. You create a draft pull request. 
5. You update the `CHANGELOG.md` file with the new language addition with pull request number
6. You mark the pull request as ready for review.
7. The maintainer of the project will ask you that you can be a maintainer of the language.

   If you want not to be a maintainer of the language,
   until someone else declares to be a maintainer, the language will not be merged.
8. The maintainer of the project will create a new discussion thread for the language.

   The discussion thread will be used for track missing or excess keys for the language.
   The GitHub Actions will update the discussion and replies to a specific thread if there is update, 
   so please track the thread if you are a maintainer of the language.
9. The maintainer of the project will update the actions script to add the language to the CI/CD process.

   For this process, please enable "Allow Edits from Maintainers" in the pull request.
10. The maintainer of the project will merge the pull request.
