import {MagnifyingGlassIcon} from "@heroicons/react/24/solid";
import React from "react";
import {useTranslation} from "react-i18next";
import {Input} from "@/components/ui/input";

export function SearchBox({className, value, onChange}: {
	className?: string,
	value?: string,
	onChange?: (e: React.ChangeEvent<HTMLInputElement>) => void
}) {
	const {t} = useTranslation();

	return (
		<div className={`relative flex gap-2 ${className}`}>
			{/* The search box */}
			<Input
				type="search"
				placeholder={t("search:placeholder")}
				className={"w-full border-t-primary placeholder:text-primary focus:border-primary pl-9 placeholder:opacity-100"}
				value={value}
				onChange={onChange}
			/>
			<MagnifyingGlassIcon className="!absolute left-3 top-[13px]" width={13} height={14}/>
		</div>
	)
}
