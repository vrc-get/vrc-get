#!/usr/bin/env node

const inputFile = process.argv[2];
const outputFile = process.argv[3];

console.log(`Converting "${inputFile}" to "${outputFile}"`);

const sharp = require("sharp");
sharp(inputFile).toFile(outputFile);

console.log("Conversion done");
