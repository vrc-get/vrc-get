// autocomplete based on shadcn combobox
// https://ui.shadcn.com/docs/components/combobox

"use client";

import * as React from "react";
import { useEffect, useMemo } from "react";
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

export type AutoCompleteOption =
	| string
	| {
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
	placeholder,
	emptyContent,
	className,
}: {
	options: AutoCompleteOptions;
	value?: string;
	onChange?: (value: string) => void;
	placeholder?: string;
	emptyContent?: React.ReactNode;
	className?: string;
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
		<Command className={className}>
			<Popover open={open} onOpenChange={setOpen} modal>
				<PopoverTrigger asChild>
					<CommandInputRaw asChild>
						<Input
							placeholder={placeholder}
							value={value}
							onChange={(v) => {
								setValue(v.currentTarget.value);
								setOpen(true);
							}}
							onKeyDown={(e) => {
								// Always allow Home/End keys to move cursor, never use them for suggestion navigation
								if (e.key === "Home" || e.key === "End") {
									// Stop the event from reaching the Command component
									e.stopPropagation();
								}
								// Allow Up/Down keys to move cursor when suggestions are not open
								if (!open && (e.key === "ArrowUp" || e.key === "ArrowDown")) {
									// Stop the event from reaching the Command component
									e.stopPropagation();
								}
							}}
						/>
					</CommandInputRaw>
				</PopoverTrigger>
				<PopoverContent
					className="p-0"
					onOpenAutoFocus={(e) => e.preventDefault()}
				>
					<CommandList>
						<CommandGroup>
							{options.map((option) => {
								if (typeof option === "string") option = { value: option };
								return (
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
								);
							})}
						</CommandGroup>
						<CommandEmpty>
							{emptyContent ?? <CallOnMount onMount={() => setOpen(false)} />}
						</CommandEmpty>
					</CommandList>
				</PopoverContent>
			</Popover>
		</Command>
	);
}

function CallOnMount({ onMount }: { onMount: () => void }) {
	useEffect(() => {
		const timeout = setTimeout(() => {
			onMount();
		}, 0);
		return () => clearTimeout(timeout);
	}, [onMount]);
	return null;
}
