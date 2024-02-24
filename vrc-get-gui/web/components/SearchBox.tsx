import {Input} from "@material-tailwind/react";
import {MagnifyingGlassIcon} from "@heroicons/react/24/solid";
import React from "react";

export function SearchBox({className, value, onChange} : {className?: string, value?: string, onChange?: (e: React.ChangeEvent<HTMLInputElement>) => void}) {
	return (

		<div className={`relative flex gap-2 ${className}`}>
			{/* The search box */}
			<Input
				type="search"
				placeholder="Search"
				containerProps={{
					className: "min-w-[100px]",
				}}
				className=" !border-t-blue-gray-300 pl-9 placeholder:text-blue-gray-300 focus:!border-blue-gray-300"
				labelProps={{
					className: "before:content-none after:content-none",
				}}
				value={value}
				onChange={onChange}
			/>
			<MagnifyingGlassIcon className="!absolute left-3 top-[13px]" width={13} height={14}/>
		</div>
	)
}
