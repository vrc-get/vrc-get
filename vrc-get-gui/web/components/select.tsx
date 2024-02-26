// based on https://github.com/creativetimofficial/material-tailwind/blob/main/packages/material-tailwind-react/src/components/Select/index.tsx#L298

import React, {createContext, useContext, useState} from "react";
import {Menu, MenuHandler, MenuItem, MenuList, useTheme} from "@material-tailwind/react";
import findMatch from "@material-tailwind/react/utils/findMatch";
import objectsToString from "@material-tailwind/react/utils/objectsToString";
import {twMerge} from "tailwind-merge";
import classnames from "classnames";
import {ChevronDownIcon} from "@heroicons/react/24/solid";

interface SelectContext {
	onClick(value: string): void;
}

export const SelectContext = createContext<SelectContext | undefined>(undefined);

export function VGSelect(
	{
		children,
		disabled,
		value,
		className,
		onChange,
	}: {
		children: React.ReactNode,
		disabled?: boolean,
		value?: React.ReactNode,
		className?: string,
		onChange?: (value: string) => void,
	}
) {
	const [state, setState] = useState<string>("close");
	const [open, setOpen] = React.useState(false);

	const {select} = useTheme();
	const {defaultProps, valid, styles} = select;
	const {base, variants} = styles;

	const contextValue: SelectContext = {
		onClick(value: string) {
			onChange?.(value);
			setOpen(false);
		}
	}

	const size = defaultProps.size;

	const selectVariant = variants.outlined;
	const selectSize = selectVariant.sizes[findMatch(valid.sizes, size, "md")];
	const stateClasses = selectVariant.states[state];
	const containerClasses = classnames(
		objectsToString(base.container),
		objectsToString(selectSize.container),
	);
	const selectClasses = twMerge(
		classnames(
			objectsToString(base.select),
			objectsToString(selectVariant.base.select),
			objectsToString(stateClasses.select),
			objectsToString(selectSize.select),
		),
		className,
	);
	const arrowClasses = classnames(objectsToString(base.arrow.initial), {
		[objectsToString(base.arrow.active)]: open,
	});
	const buttonContentClasses = "absolute top-2/4 -translate-y-2/4 left-3 pt-0.5";

	React.useEffect(() => {
		if (open) {
			setState("open");
		} else {
			setState("close");
		}
	}, [open, value]);

	return (
		<SelectContext.Provider value={contextValue}>
			<Menu open={open} handler={() => setOpen(!open)}>
				<div className={containerClasses}>
					<MenuHandler>
						<button className={selectClasses} disabled={disabled}>
							<span className={buttonContentClasses}>{value}</span>
							<div className={arrowClasses}>
								<ChevronDownIcon className="size-3"/>
							</div>
						</button>
					</MenuHandler>
				</div>
				<MenuList className={"max-h-96 overflow-y-scroll"}>
					{children}
				</MenuList>
			</Menu>
		</SelectContext.Provider>
	)
}

export const VGOption = React.forwardRef(VGOptionImpl)

function VGOptionImpl(
	{
		children,
		value,
	}: {
		children: React.ReactNode,
		value: string,
	},
	ref: React.Ref<HTMLButtonElement>
) {
	const contextValue = useContext(SelectContext);
	return (
		<MenuItem ref={ref} onClick={() => contextValue?.onClick(value)}>{children}</MenuItem>
	)
}
