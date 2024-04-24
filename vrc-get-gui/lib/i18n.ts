import i18next, {t as i18nextt} from "i18next";
import {initReactI18next} from "react-i18next";
import enJson from "@/locales/en.json5";
import deJson from "@/locales/de.json5";
import jaJson from "@/locales/ja.json5";
import zh_cnJson from "@/locales/zh_cn.json5";
import {tc as tcOriginal} from "@/components/VGTrans";

i18next
	.use(initReactI18next)
	.init({
		resources: {
			en: enJson,
			de: deJson,
			ja: jaJson,
			zh_cn: zh_cnJson,
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
				'b',
				'i',
				'code',
			]
		}
	})

export default i18next;
export const languages = [
	"en",
	"de",
	"ja",
	"zh_cn",
];

export const tc = tcOriginal;

export const tt = i18nextt;
