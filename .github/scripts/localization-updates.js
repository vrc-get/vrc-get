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
	const enJson = json5.parse(await fs.readFile(`vrc-get-gui/locales/en.json5`, "utf8"));
	const enKeys = normalizeKeys(Object.keys(enJson.translation));
	const transJson = json5.parse(await fs.readFile(`vrc-get-gui/locales/${localeId}.json5`, "utf8"));
	const transKeys = normalizeKeys(Object.keys(transJson.translation));

	const {missingList: missingKeys, extraList: extraKeys} = missingAndExtras(enKeys, transKeys);

	const newData = {
		missingKeys,
		extraKeys,
	};

	const newAutoPart = `**Missing Keys:**\n${listToMarkdown(missingKeys)}\n\n**Excess Keys:**\n${listToMarkdown(extraKeys)}\n`;

	const {discussionId, previousJson: dataJson} = await updateComment(github, owner, repo, number, newAutoPart, newData);
	dataJson.missingKeys ??= [];
	dataJson.extraKeys ??= [];

	// create comment if there are new missing / extra keys
	const {extraList: newlyAddedMissingKeys} = missingAndExtras(normalizeKeys(dataJson.missingKeys), missingKeys);
	const {extraList: newlyAddedExtraKeys} = missingAndExtras(normalizeKeys(dataJson.extraKeys), extraKeys);
	if (newlyAddedMissingKeys.length > 0 || newlyAddedExtraKeys.length > 0) {
		const text = `
There are new missing / excess keys in the translation. Please update the translation!

**New Missing Keys:**

${listToMarkdown(newlyAddedMissingKeys)}

**New Excess Keys:**

${listToMarkdown(newlyAddedExtraKeys)}
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
 * @template T
 * @param beforeList {T[]}
 * @param afterList {T[]}
 * @return {{missingList: T[], extraList: T[]}}
 */
function missingAndExtras(beforeList, afterList) {
	const missingList = beforeList.filter(key => !afterList.includes(key)).filter(key => !optionalKeys.includes(key));
	const extraList = afterList.filter(key => !beforeList.includes(key)).filter(key => !optionalKeys.includes(key));

	return {missingList, extraList};
}

function listToMarkdown(values) {
	return values.length === 0 ? 'nothing' : values.map(key => `- \`${key}\``).join('\n')
}

/**
 *
 * @param github {import('@octokit/rest').Octokit}
 * @param owner {string}
 * @param repo {string}
 * @param number {number}
 * @param content {string} the updated content
 * @param newData {object} data stored in the comment
 * @return {Promise<{previousJson: object, discussionId:string}>}
 */
async function updateComment(
	github,
	owner, repo, number,
	content,
	newData,
) {
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
	const previousJson = dataJsonLine ? JSON.parse(dataJsonLine.slice(dataJsonLinePrefix.length)) : {};

	const newBody = `${manualPart}${autoPartStart}${content}
<!-- data part
${dataJsonLinePrefix}${JSON.stringify(newData)}
-->
${autoPartEnd}${postAutoPart}`;

	await github.graphql(`
		mutation($discussionId: ID!, $body: String!) {
			updateDiscussion(input: {discussionId: $discussionId, body: $body}) {
				discussion {
					body
				}
			}
		}
	`, {discussionId, body: newBody});

	return {
		previousJson,
		discussionId,
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
