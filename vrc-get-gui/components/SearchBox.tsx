import { Search } from "lucide-react";
import type React from "react";
import { useTranslation } from "react-i18next";
import { Input } from "@/components/ui/input";

type SearchBoxProps = {
	className?: string;
	inputClassName?: string;
	iconClassName?: string;
	value?: string;
	onChange?: (e: React.ChangeEvent<HTMLInputElement>) => void;
	ref?: React.Ref<HTMLInputElement>;
};

export const SearchBox = function SearchBox({
	className,
	inputClassName,
	iconClassName,
	value,
	onChange,
	ref,
}: SearchBoxProps) {
	const { t } = useTranslation();

	return (
		<div className={`relative flex gap-2 ${className}`}>
			{/* The search box */}
			<Input
				type="search"
				placeholder={t("search:placeholder")}
				className={`
					w-full placeholder:text-primary focus:border-primary pl-9 placeholder:opacity-100
					${inputClassName}
				`}
				value={value}
				onChange={onChange}
				ref={ref}
			/>
			<Search
				className={`absolute! left-4 top-[17px] compact:top-[13px] ${iconClassName}`}
				width={13}
				height={14}
			/>
		</div>
	);
};
