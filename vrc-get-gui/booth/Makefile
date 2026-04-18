npm.install:
	npm install

app-icon.png: ../app-icon.png
	cp ../app-icon.png app-icon.png

thumbnail.png: thumbnail.svg alcom-dark.png alcom-light.png app-icon.png
	rsvg-convert thumbnail.svg -o thumbnail.png

booth.zip: website.url README.ja.txt README.en.txt LICENSE
	zip -r $@ $^
