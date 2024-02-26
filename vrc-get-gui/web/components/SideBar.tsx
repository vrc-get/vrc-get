"use client";

import {Card, List, ListItem, ListItemPrefix} from "@material-tailwind/react";
import {Cog6ToothIcon, ListBulletIcon,} from "@heroicons/react/24/solid";
import React from "react";

export function SideBar({className}: { className?: string }) {
	"use client"

	return (
		<Card className={`${className} w-auto max-w-[20rem] p-4 shadow-xl shadow-blue-gray-900/5 h-screen shrink-0`}>
			<List className="min-w-[10rem]">
				<SideBarItem href={"/projects"} text={"Projects"} icon={ListBulletIcon}/>
				<SideBarItem href={"/settings"} text={"Settings"} icon={Cog6ToothIcon}/>
			</List>
		</Card>
	);
}

function SideBarItem(
	{href, text, icon}: { href: string, text: string, icon: React.ComponentType<{className?: string}> }
) {
	const IconElenment = icon;
	return (
		<ListItem onClick={() => location.href = href}>
			<ListItemPrefix>
				<IconElenment className="h-5 w-5"/>
			</ListItemPrefix>
			{text}
		</ListItem>
	);
}
