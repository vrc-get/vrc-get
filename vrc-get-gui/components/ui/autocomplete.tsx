// autocomplete based on shadcn combobox
// https://ui.shadcn.com/docs/components/combobox

"use client";

import * as React from "react";
import { useMemo } from "react";
import {
	Command,
	CommandEmpty,
	CommandGroup,
	CommandInputRaw,
	CommandItem,
	CommandList,
} from "@/components/ui/command";
import { Input } from "@/components/ui/input";
import {
	Popover,
	PopoverContent,
	PopoverTrigger,
} from "@/components/ui/popover";

type AutoCompleteOption = {
	value: string;
	key?: string;
	label?: React.ReactNode;
	keywords?: string[];
};

type AutoCompleteOptions =
	| AutoCompleteOption[]
	| ((value: string) => AutoCompleteOption[]);

export function Autocomplete({
	options: optionsIn,
	value: valueIn,
	onChange: onChangeIn,
	emptyContent,
}: {
	options: AutoCompleteOptions;
	value?: string;
	onChange?: (value: string) => void;
	emptyContent?: React.ReactNode;
}) {
	const isControlled = valueIn != null;

	const [open, setOpen] = React.useState(false);
	const [stateValue, setStateValue] = React.useState("");

	const value = isControlled ? valueIn : stateValue;
	const setValue = isControlled ? (onChangeIn ?? (() => {})) : setStateValue;

	const options = useMemo(
		() => (typeof optionsIn === "function" ? optionsIn(value) : optionsIn),
		[value, optionsIn],
	);

	return (
		<Command>
			<Popover open={open} onOpenChange={setOpen}>
				<PopoverTrigger asChild>
					<CommandInputRaw asChild>
						<Input
							placeholder="Search framework..."
							value={value}
							onChange={(v) => setValue(v.currentTarget.value)}
						/>
					</CommandInputRaw>
				</PopoverTrigger>
				<PopoverContent
					className="p-0"
					onOpenAutoFocus={(e) => e.preventDefault()}
				>
					<CommandList>
						<CommandEmpty>{emptyContent}</CommandEmpty>
						<CommandGroup>
							{options.map((option) => (
								<CommandItem
									key={option.key ?? option.value}
									keywords={option.keywords}
									value={option.value}
									onSelect={(currentValue) => {
										setValue(currentValue);
										setOpen(false);
									}}
								>
									{option.label ?? option.value}
								</CommandItem>
							))}
						</CommandGroup>
					</CommandList>
				</PopoverContent>
			</Popover>
		</Command>
	);
}
