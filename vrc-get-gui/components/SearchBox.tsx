import { Input } from "@/components/ui/input";
import { Search } from "lucide-react";
import type React from "react";
import { useTranslation } from "react-i18next";

export function SearchBox({
	className,
	value,
	onChange,
}: {
	className?: string;
	value?: string;
	onChange?: (e: React.ChangeEvent<HTMLInputElement>) => void;
}) {
	const { t } = useTranslation();

	return (
		<div className={`relative flex gap-2 ${className}`}>
			{/* The search box */}
			<Input
				type="search"
				placeholder={t("search:placeholder")}
				className={
					"w-full placeholder:text-primary focus:border-primary pl-9 placeholder:opacity-100"
				}
				value={value}
				onChange={onChange}
			/>
			<Search className="!absolute left-4 top-[17px]" width={13} height={14} />
		</div>
	);
}
