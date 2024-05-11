import i18next, {t as i18nextt} from "i18next";
import {initReactI18next} from "react-i18next";
import enJson from "@/locales/en.json";
import deJson from "@/locales/de.json";
import jaJson from "@/locales/ja.json";
import zh_cnJson from "@/locales/zh_cn.json";
import frJson from "@/locales/fr.json";
import ach_ugJson from "@/locales/ach_UG.json"; // fake language for crowdin in context
import {tc as tcOriginal} from "@/components/VGTrans";

const languageResources = {
	en: enJson,
	de: deJson,
	ja: jaJson,
	fr: frJson,
	zh_cn: zh_cnJson,
	ach_ug: ach_ugJson,
}

i18next
	.use(initReactI18next)
	.init({
		resources: languageResources,
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
export const languages = Object.keys(languageResources);

export const tc = tcOriginal;

export const tt = i18nextt;
