import i18next from "i18next";
import {initReactI18next} from "react-i18next";
import enJson from "@/locales/en.json5";

i18next
	.use(initReactI18next)
	.init({
		resources: {
			en: enJson,
		},
		lng: "en",
		fallbackLng: "en",
		nsSeparator: '::',

		interpolation: {
			// react is xzz safe (in general)
			escapeValue: false,
		},
	})

export default i18next;
