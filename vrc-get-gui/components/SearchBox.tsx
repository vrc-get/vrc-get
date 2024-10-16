import { Input } from "@/components/ui/input";
import { Search } from "lucide-react";
import type React from "react";
import { forwardRef } from "react";
import { useTranslation } from "react-i18next";

type SearchBoxProps = {
	className?: string;
	value?: string;
	onChange?: (e: React.ChangeEvent<HTMLInputElement>) => void;
};

export const SearchBox = forwardRef<HTMLInputElement, SearchBoxProps>(
	function SearchBox({ className, value, onChange }, ref) {
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
					ref={ref}
				/>
				<Search
					className="!absolute left-4 top-[17px]"
					width={13}
					height={14}
				/>
			</div>
		);
	},
);
