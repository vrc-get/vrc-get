"use client"

import Script from "next/script"
import i18next from "@/lib/i18n";
import {useState} from "react";

export default function CrowdinInContext() {
    const [useJipt, setUseJipt] = useState(i18next.language === 'ach_ug')

    i18next.on('languageChanged', lang => {
        setUseJipt(lang === 'ach_ug')
    })

    return (
        useJipt ? (
            <>
                <Script src="/crowdin-in-context.js" />
                <Script src="https://cdn.crowdin.com/jipt/jipt.js" strategy="lazyOnload" />
            </>
        ) : <></>
    )
}
