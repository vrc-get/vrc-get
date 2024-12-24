
thumbnail.embed.svg: thumbnail.svg alcom-dark.png alcom-light.png ../app-icon.png
	node embeder.mjs thumbnail.embed.svg thumbnail.svg alcom-dark.png alcom-light.png ../app-icon.png

thumbnail.png: thumbnail.embed.svg
	npx sharp -i thumbnail.embed.svg -o thumbnail.png

booth.zip: website.url README.ja.txt README.en.txt LICENSE
	zip -r $@ $^
