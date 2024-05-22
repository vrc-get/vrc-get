import * as React from "react"
import { Slot } from "@radix-ui/react-slot"
import { cva, type VariantProps } from "class-variance-authority"

import { cn } from "@/lib/utils"

const buttonVariants = cva(
  "inline-flex items-center justify-center whitespace-nowrap rounded-md text-sm font-medium ring-offset-background transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 disabled:pointer-events-none disabled:opacity-50",
  {
    variants: {
      variant: {
        default: "bg-button text-button-foreground hover:bg-button/90 shadow-button/50 hover:shadow-button/50 shadow hover:shadow-md transition-shadow uppercase",
        destructive:
          "bg-destructive text-destructive-foreground hover:bg-destructive/90 shadow-destructive/50 hover:shadow-destructive/50 shadow hover:shadow-md transition-shadow uppercase",
        outline:
          "border border-input hover:text-accent-foreground",
        secondary:
          "bg-secondary text-secondary-foreground hover:bg-secondary/80 shadow-secondary/50 hover:shadow-secondary/50 shadow hover:shadow-md transition-shadow uppercase",
        ghost: "hover:bg-accent text-accent-foreground hover:text-accent-foreground",
        link: "text-button underline-offset-4 hover:underline",
        info: "bg-info text-info-foreground hover:bg-info/90 shadow-info/50 hover:shadow-info/50 shadow hover:shadow-md transition-shadow uppercase",
        success: "bg-success text-success-foreground hover:bg-success/90 shadow-success/50 hover:shadow-success/50 shadow hover:shadow-md transition-shadow uppercase",
      },
      size: {
        default: "h-10 px-4 py-2",
        sm: "h-9 rounded-md px-3",
        lg: "h-11 rounded-md px-8",
        icon: "h-10 w-10",
      },
    },
    defaultVariants: {
      variant: "default",
      size: "default",
    },
  }
)

export interface ButtonProps
  extends React.ButtonHTMLAttributes<HTMLButtonElement>,
    VariantProps<typeof buttonVariants> {
  asChild?: boolean
}

const Button = React.forwardRef<HTMLButtonElement, ButtonProps>(
  ({ className, variant, size, asChild = false, ...props }, ref) => {
    const Comp = asChild ? Slot : "button"
    return (
      <Comp
        className={cn(buttonVariants({ variant, size, className }))}
        ref={ref}
        {...props}
      />
    )
  }
)
Button.displayName = "Button"

export { Button, buttonVariants }
