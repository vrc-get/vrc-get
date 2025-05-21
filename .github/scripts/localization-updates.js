const fs = require('node:fs/promises');
const json5 = require('json5');

// Those keys are optional; they are not required to be translated and might be translated only by some locales.
const optionalKeys = [
	"projects:manage:dialog:downgrade major vrchat supported",
	"projects:manage:dialog:downgrade major vrchat unsupported",
	"projects:manage:dialog:downgrade minor vrchat supported",
	"projects:manage:dialog:downgrade minor vrchat unsupported",
	"projects:manage:dialog:upgrade minor vrchat supported",
	"projects:manage:dialog:upgrade minor vrchat unsupported",
	"projects:manage:dialog:upgrade major vrchat supported",
	"projects:manage:dialog:upgrade major vrchat unsupported",
]

/**
 * @param github {import('@octokit/rest').Octokit}
 * @param context {{repo: {owner: string, repo: string}}}
 */
module.exports = async ({github, context}) => {
	if (context.repo.owner !== 'vrc-get') return;

	const {owner, repo} = context.repo;

	const locales = [
		{
			id: 'ja',
			discussionNumber: 855,
			replyId: 'DC_kwDOIza9ks4AjSve',
		},
		{
			id: 'de',
			discussionNumber: 860,
			replyId: 'DC_kwDOIza9ks4AjS5I',
		},
		{
			id: 'zh_hans',
			discussionNumber: 888,
			replyId: 'DC_kwDOIza9ks4AjUwo',
		},
		{
			id: 'fr',
			discussionNumber: 909,
			replyId: 'DC_kwDOIza9ks4Aji4V',
		},
		{
			id: 'zh_hant',
			discussionNumber: 1443,
			replyId: 'DC_kwDOIza9ks4An6A8'
		},
		{
			id: 'ko',
			discussionNumber: 1823,
			replyId: 'DC_kwDOIza9ks4AswKE'
		},
	];

	for (const locale of locales) {
		await processOneLocale(github, owner, repo, locale.discussionNumber, locale.replyId, locale.id);
	}
}

/**
 *
 * @param github {import('@octokit/rest').Octokit}
 * @param owner {string}
 * @param repo {string}
 * @param number {number}
 * @param replyToId {string}
 * @param localeId {string}
 * @return {Promise<void>}
 */
async function processOneLocale(github, owner, repo, number, replyToId, localeId) {
	/** @type {{data: {repository: {discussion: {body: string}}}}} */
	const result = await github.graphql(`
		query($owner: String!, $repo: String!, $number: Int!) {
			repository(owner: $owner, name: $repo) {
				discussion(number: $number) {
					body
					id
				}
			}
		}
		`, {owner, repo, number})

	const body = result.repository.discussion.body;
	const discussionId = result.repository.discussion.id;

	const dataJsonLinePrefix = "prevData: ";
	const autoPartStart = "<!-- github actions update start -->";
	const autoPartEnd = "<!-- github actions update end -->";

	const split = body.split(autoPartStart, 2);
	const manualPart = split[0];
	const temp = split[1] ?? '';
	const split1 = temp.split(autoPartEnd, 2);
	const autoPart = split1[0];
	const postAutoPart = split1[1] ?? '';

	const dataJsonLine = autoPart.split(/\r?\n/).find(l => l.startsWith(dataJsonLinePrefix));
	// the dataJson is for computing the difference and create new comment if there are changes
	const dataJson = dataJsonLine ? JSON.parse(dataJsonLine.slice(dataJsonLinePrefix.length)) : {};
	dataJson.missingKeys ??= [];
	dataJson.extraKeys ??= [];

	const enJson = json5.parse(await fs.readFile(`vrc-get-gui/locales/en.json5`, "utf8"));
	const enKeys = normalizeKeys(Object.keys(enJson.translation));
	const transJson = json5.parse(await fs.readFile(`vrc-get-gui/locales/${localeId}.json5`, "utf8"));
	const transKeys = normalizeKeys(Object.keys(transJson.translation));

	const missingKeys = enKeys.filter(key => !transKeys.includes(key)).filter(key => !optionalKeys.includes(key));
	const extraKeys = transKeys.filter(key => !enKeys.includes(key)).filter(key => !optionalKeys.includes(key));

	const missingKeysStr = missingKeys.length === 0 ? 'nothing' : missingKeys.map(key => `- \`${key}\``).join('\n');
	const excessKeysStr = extraKeys.length === 0 ? 'nothing' : extraKeys.map(key => `- \`${key}\``).join('\n');

	const newData = {
		missingKeys,
		extraKeys,
	};

	const newAutoPart = `
**Missing Keys:**

${missingKeysStr}

**Excess Keys:**

${excessKeysStr}

<!-- data part
${dataJsonLinePrefix}${JSON.stringify(newData)}
-->
`;

	const newBody = `${manualPart}${autoPartStart}${newAutoPart}${autoPartEnd}${postAutoPart}`;

	await github.graphql(`
		mutation($discussionId: ID!, $body: String!) {
			updateDiscussion(input: {discussionId: $discussionId, body: $body}) {
				discussion {
					body
				}
			}
		}
	`, {discussionId, body: newBody});

	// create comment if there are new missing / extra keys
	const oldMissingKeys = new Set(normalizeKeys(dataJson.missingKeys));
	const oldExtraKeys = new Set(normalizeKeys(dataJson.extraKeys));
	const newlyAddedMissingKeys = missingKeys.filter(key => !oldMissingKeys.has(key));
	const newlyAddedExtraKeys = extraKeys.filter(key => !oldExtraKeys.has(key));
	if (newlyAddedMissingKeys.length > 0 || newlyAddedExtraKeys.length > 0) {
		const newMissingKeysStr = newlyAddedMissingKeys.length === 0 ? 'nothing' : newlyAddedMissingKeys.map(key => `- \`${key}\``).join('\n');
		const newExcessKeysStr = newlyAddedExtraKeys.length === 0 ? 'nothing' : newlyAddedExtraKeys.map(key => `- \`${key}\``).join('\n');

		const text = `
There are new missing / excess keys in the translation. Please update the translation!

**New Missing Keys:**

${newMissingKeysStr}

**New Excess Keys:**

${newExcessKeysStr}
`

		await github.graphql(`
			mutation($discussionId: ID!, $replyToId: ID!, $body: String!) {
				addDiscussionComment(input: {discussionId: $discussionId, replyToId: $replyToId, body: $body}) {
					comment {
						body
					}
				}
			}
		`, {discussionId, replyToId, body: text});
	}
}

/**
 * @param keys {string[]}
 * @return {string[]}
 */
function normalizeKeys(keys) {
	return keys.map(k => k.replace(/_(one|other)/, ''));
}

if (require.main === module) {
	const {Octokit} = require('@octokit/rest');
	module.exports({
		github: new Octokit({auth: process.env.GITHUB_TOKEN}),
		context: {
			repo: {
				owner: process.env.REPO_OWNER,
				repo: process.env.REPO_NAME,
			},
		},
	});
}
