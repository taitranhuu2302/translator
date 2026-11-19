/**
 * The Neris mark: two interlocking chevrons that read as an N.
 *
 * It is the same geometry as the app icon and the menu bar template
 * (`crates/app/icons/`), drawn in `currentColor` so the surface decides its
 * weight. It is not a Lucide icon and does not follow their stroke rules: this
 * is a filled logotype, so it is sized by `className` and never recoloured by a
 * parent's `text-*` except on purpose.
 */
export function NerisMark({ className }: { className?: string }) {
  return (
    <svg
      aria-hidden="true"
      className={className}
      fill="currentColor"
      viewBox="255 278 1490 1443"
      xmlns="http://www.w3.org/2000/svg"
    >
      <g transform="matrix(6.959631,0,0,6.959631,-10716.36761,-7829.757728)">
        <g transform="matrix(1,0,0,1,-9.317851,-9.563058)">
          <path d="M1638.338,1306.161L1638.338,1249.477L1747.249,1329.198L1747.249,1307.071L1798.566,1344.578L1798.566,1379.945C1798.566,1381.005 1797.706,1381.865 1796.646,1381.865C1786.595,1381.865 1740.501,1381.865 1740.501,1381.865L1638.338,1306.161ZM1638.338,1306.161L1638.338,1381.739C1638.338,1381.739 1597.353,1381.838 1587.91,1381.86C1587.4,1381.861 1586.91,1381.66 1586.549,1381.3C1586.188,1380.939 1585.985,1380.45 1585.985,1379.94L1585.985,1266.614L1638.338,1306.161Z" />
        </g>
        <g transform="matrix(-1,0,0,-1,3376.268643,2546.984276)">
          <path d="M1638.338,1306.161L1638.338,1249.477L1747.249,1329.198L1747.249,1307.071L1798.566,1344.578L1798.566,1379.945C1798.566,1381.005 1797.706,1381.865 1796.646,1381.865C1786.595,1381.865 1740.501,1381.865 1740.501,1381.865L1638.338,1306.161ZM1638.338,1306.161L1638.338,1381.739C1638.338,1381.739 1597.353,1381.838 1587.91,1381.86C1587.4,1381.861 1586.91,1381.66 1586.549,1381.3C1586.188,1380.939 1585.985,1380.45 1585.985,1379.94L1585.985,1266.614L1638.338,1306.161Z" />
        </g>
      </g>
    </svg>
  );
}
