import {Trans, useTranslation} from "react-i18next";
import {TransProps} from "react-i18next/TransWithoutContext";
import React from "react";
import dynamic from "next/dynamic";


// localization would cause hydration error so do not ssr
export const VGTrans = dynamic(() => Promise.resolve(function VGTrans(props: TransProps<string>) {
	const {t} = useTranslation();

	return <Trans
		{...props}
		t={t}
	/>
}), {ssr: false});

export function tc(key: string | string[], values?: { [key: string]: string | number }, props?: TransProps<string>) {
	return <VGTrans
		i18nKey={key}
		values={values}
		{...props}
	/>
}
