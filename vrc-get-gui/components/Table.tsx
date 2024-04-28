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
        layout?: string[],
        header: React.ReactNode[]
        rows: React.ReactNode[][]
}) => {
    if (!layout){
        layout = Array(header.length).fill(0).map(_ => "auto")
    }
    return (<div className={`grid overflow-x-auto rounded-xl border border-blue-gray-100 ${className}`} style={{gridTemplateColumns: layout.join(" ") }}>
        {/* Header */}
        {header.map((headerItem, headerIndex) => (<div
            key={headerIndex}
            className={
                `border-blue-gray-100 bg-blue-gray-100 py-2.5 px-4 ` + 
                (headerIndex === 0 ? " rounded-tl-xl " : "") +
                (headerIndex === header.length -1 ?  "rounded-tr-xl " : "") +
                (headerIndex === 0 && rows.length === 0 ? " rounded-bl-xl " : "") +
                (headerIndex === header.length - 1 && rows.length === 0 ? " rounded-br-xl " : "")
            } 
        >
            <Typography variant="small" className="font-normal leading-none">{headerItem}</Typography>
        </div>))}
        {/* Rows */}
        {rows.map((row, rowIndex) => row.map((rowItem, rowItemIndex) => (<div
            key={`${rowIndex}-${rowItemIndex}`}
            className={"py-2.5 px-4 text-ellipsis flex items-center overflow-hidden " + (rowIndex % 2 !== 0 && " border-b border-blue-gray-100 bg-blue-gray-50 " + (rowIndex === rows.length - 1 && rowItemIndex === 0 && " rounded-bl-xl ") + (rowIndex === rows.length - 1 && rowItemIndex === header.length -1 && " rounded-br-xl ") )}
        >
            {rowItem}
        </div>)))}
        
    </div>)
}

export default Table