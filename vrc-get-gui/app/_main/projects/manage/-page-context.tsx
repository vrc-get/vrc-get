import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import { type ComponentProps, createContext, useContext } from "react";

interface PageContext {
	isLoading: boolean;
}

export const PageContext = createContext<PageContext>({
	isLoading: false,
});
PageContext.displayName = "PageContext";

export const PageContextProvider = PageContext.Provider;

export function usePageContext() {
	return useContext(PageContext);
}

export const ButtonDisabledIfLoading = function ButtonDisabledIfLoading({
	disabled,
	...props
}: ComponentProps<typeof Button>) {
	const { isLoading } = usePageContext();
	return <Button disabled={isLoading || disabled} {...props} />;
};

export const CheckboxDisabledIfLoading = function CheckboxDisabledIfLoading({
	disabled,
	...props
}: ComponentProps<typeof Checkbox>) {
	const { isLoading } = usePageContext();
	return <Checkbox disabled={isLoading || disabled} {...props} />;
};
