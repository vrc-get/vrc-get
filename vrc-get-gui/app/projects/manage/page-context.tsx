import React, {ComponentProps, createContext, useContext} from "react";
import {Button} from "@/components/ui/button";
import {Checkbox} from "@/components/ui/checkbox";

interface PageContext {
  isLoading: boolean;
}

export const PageContext = createContext<PageContext>({
  isLoading: false,
})
PageContext.displayName = "PageContext";

export const PageContextProvider = PageContext.Provider;

export function usePageContext() {
  return useContext(PageContext);
}

export function ButtonDisabledIfLoading(
  {
    disabled,
    ...props
  }: ComponentProps<typeof Button>,
) {
  const {isLoading} = usePageContext();
  console.log(`rerender: ButtonDisabledIfLoading: isloading: ${isLoading}`);
  return <Button disabled={isLoading || disabled} {...props} />
}

export function CheckboxDisabledIfLoading(
  {
    disabled,
    ...props
  }: ComponentProps<typeof Checkbox>,
) {
  const {isLoading} = usePageContext();
  console.log(`rerender: CheckboxDisabledIfLoading: isloading: ${isLoading}`);
  return <Checkbox disabled={isLoading || disabled} {...props}/>
}
