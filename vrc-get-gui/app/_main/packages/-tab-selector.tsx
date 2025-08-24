import { Link } from "@tanstack/react-router";
import { tc } from "@/lib/i18n";

type PageType =
	| "/packages/user-packages"
	| "/packages/repositories"
	| "/packages/templates";

// Note: For historical reasons, templates page are under packages in route.

export function HeadingPageName({ pageType }: { pageType: PageType }) {
	// Note for p-1 rounded-md -m-1 compact:m-0
	// For normal mode, we use 1-unit of the outer padding for selector rectangle, so we use negative margin to eat padding.
	// For compact mode, the height of the button is 2 units shorter than normal with the height of the navbar is remaining.
	// Therefore we use the 1 unit space for outer padding for selector rectangle.
	return (
		<div className={"flex compact:h-10 items-center"}>
			<div
				className={
					"grid grid-cols-3 gap-1.5 bg-secondary p-1 rounded-md -m-1 compact:m-0"
				}
			>
				<HeadingButton
					currentPage={pageType}
					targetPage={"/packages/repositories"}
				>
					{tc("packages:repositories")}
				</HeadingButton>
				<HeadingButton
					currentPage={pageType}
					targetPage={"/packages/user-packages"}
				>
					{tc("packages:user packages")}
				</HeadingButton>
				<HeadingButton
					currentPage={pageType}
					targetPage={"/packages/templates"}
				>
					{tc("packages:templates")}
				</HeadingButton>
			</div>
		</div>
	);
}

function HeadingButton({
	currentPage,
	targetPage,
	children,
}: {
	currentPage: PageType;
	targetPage: PageType;
	children: React.ReactNode;
}) {
	const button =
		"cursor-pointer px-3 py-2 font-bold grow-0 hover:bg-background rounded-sm text-center p-2 compact:h-8 compact:py-1";

	if (currentPage === targetPage) {
		return <div className={`${button} bg-background`}>{children}</div>;
	} else {
		return (
			<Link to={targetPage} className={button}>
				{children}
			</Link>
		);
	}
}
