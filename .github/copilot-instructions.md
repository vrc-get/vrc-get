If you're writing code:
- Please don't make localization for locales other than en / ja. I cannot review those locales.
- Run cargo clippy for lints and cargo fmt for format before commit.
- After completing the code and commit, please add a changelog entry. Please note that the numbers in the changelog file are pull request numbers, not issue numbers.
    - Please add it to the bottom of the change list.
    - Please use the proper section for each change. "Fix" should be used only for bug fixes. UX improvements typically belong under "Change", and new features typically under "Add". These are not strict rules, so use them flexibly.
- You should use Conventional Commits (chore:, fix:, dev:, build:, docs:, style:, lint:, and others).
- Please split commits for implementation and changelog updates.
