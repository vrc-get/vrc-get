import i18next, {t as i18nextt} from "i18next";
import {initReactI18next} from "react-i18next";
import enJson from "@/locales/en.json5";
import jaJson from "@/locales/ja.json5";
import {tc as tcOriginal} from "@/components/VGTrans";

i18next
	.use(initReactI18next)
	.init({
		resources: {
			en: enJson,
			ja: jaJson,
		},
		lng: "en",
		fallbackLng: "en",
		nsSeparator: '::',

		interpolation: {
			// react is xzz safe (in general)
			escapeValue: false,
		},
		react: {
			transKeepBasicHtmlNodesFor: [
				'br',
				'strong',
				'i',
				'code',
			]
		}
	})

export default i18next;
export const languages = [
	"en",
	"ja",
];

export const tc = tcOriginal;

export const tt = i18nextt;
