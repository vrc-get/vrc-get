import * as fs from "node:fs/promises";

const output = process.argv[2];
const input = process.argv[3];

let inputText = await fs.readFile(input, { encoding: "utf-8" });

for (let i = 4; i < process.argv.length; i++) {
	const embedPath = process.argv[i];
	const embedDataBase64 = await fs.readFile(embedPath, { encoding: "base64" });
	const embedDataUrl = `data:image/png;base64,${embedDataBase64}`;
	inputText = inputText.replace(`"${embedPath}"`, `"${embedDataUrl}"`);
}

await fs.writeFile(output, inputText);
