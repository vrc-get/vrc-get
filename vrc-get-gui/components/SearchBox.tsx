import { Search } from "lucide-react";
import type React from "react";
import { useTranslation } from "react-i18next";
import { Input } from "@/components/ui/input";
import { cn } from "@/lib/utils";

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
	value,
	onChange,
	ref,
}: SearchBoxProps) {
	const { t } = useTranslation();

	return (
		<div className={cn(`relative flex gap-2 h-10 compact:h-8`, className)}>
			<Input
				type="search"
				placeholder={t("search:placeholder")}
				className={`
					w-full placeholder:text-primary focus:border-primary pl-9 placeholder:opacity-100
					h-full compact:h-full
				`}
				value={value}
				onChange={onChange}
				ref={ref}
			/>
			<Search
				className={`absolute! left-4 top-[calc(50%-7px)]`}
				width={13}
				height={14}
			/>
		</div>
	);
};
