"use client"

import Script from "next/script"

export default function CrowdinInContext() {
  return (
    <>
      <Script src="/crowdin-in-context.js" />
      <Script src="https://cdn.crowdin.com/jipt/jipt.js" />
    </>
  )
}
