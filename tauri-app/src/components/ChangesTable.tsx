import { ChangeEntry } from "../types";
import "./ChangesTable.css";

interface ChangesTableProps {
  changes: ChangeEntry[];
}

export function ChangesTable({ changes }: ChangesTableProps) {
  if (changes.length === 0) {
    return null;
  }

  return (
    <div className="changes-table-container">
      <table className="changes-table">
        <thead>
          <tr>
            <th>Plugin</th>
            <th>Parameter</th>
            <th>Old</th>
            <th>→</th>
            <th>New</th>
            <th>Reason</th>
          </tr>
        </thead>
        <tbody>
          {changes.map((change, index) => (
            <tr key={index} className="change-row">
              <td className="plugin-name">{change.plugin}</td>
              <td className="param-name">{change.parameter}</td>
              <td className="old-value">{change.old_value}</td>
              <td className="arrow">→</td>
              <td className="new-value">{change.new_value}</td>
              <td className="reason">{change.reason}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
