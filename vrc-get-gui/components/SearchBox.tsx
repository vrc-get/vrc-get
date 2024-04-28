import {MagnifyingGlassIcon} from "@heroicons/react/24/solid";
import React from "react";
import {useTranslation} from "react-i18next";
import {InputNoLabel} from "@/components/InputNoLabel";

export function SearchBox({className, value, onChange}: {
	className?: string,
	value?: string,
	onChange?: (e: React.ChangeEvent<HTMLInputElement>) => void
}) {
	const {t} = useTranslation();

	return (
		<div className={`relative flex gap-2 ${className}`}>
			{/* The search box */}
			<InputNoLabel
				type="search"
				placeholder={t("search:placeholder")}
				className={"pl-9 placeholder:opacity-100"}
				value={value}
				onChange={onChange}
			/>
			<MagnifyingGlassIcon className="!absolute left-3 top-[13px]" width={13} height={14}/>
		</div>
	)
}
