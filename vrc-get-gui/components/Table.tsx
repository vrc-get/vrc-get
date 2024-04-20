import { Card, Typography } from "@material-tailwind/react";
import React from "react";

const Table = (
	{
		className,
        layout,
        header,
        rows
	}: {
        className?: string,
        layout: string[],
        header: React.ReactNode[]
        rows: React.ReactNode[][]
}) => {
    return (<Card className={`grid ${className}`} style={{gridTemplateColumns: layout.join(" ") }}>
        {/* Header */}
        {header.map((headerItem, headerIndex) => (<div
            key={headerIndex}
            className="border-blue-gray-100 bg-blue-gray-50 py-2.5 px-4"
        >
            <Typography variant="small" className="font-normal leading-none">{headerItem}</Typography>
        </div>))}
        {/* Rows */}
        {rows.map((row, rowIndex) => row.map((rowItem, rowItemIndex) => (<div
            key={`${rowIndex}-${rowItemIndex}`}
            className="py-2.5 px-4 text-ellipsis overflow-hidden"
        >
            <Typography variant="paragraph" className="text-ellipsis">{rowItem}</Typography>
        </div>)))}
        
    </Card>)
}

export default Table