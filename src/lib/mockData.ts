import type { Task } from "./types";

export const initialTasks: Task[] = [
  {
    id: "1",
    title: "Reply to Sam about the roadmap doc",
    note: "",
    bucket: "inbox",
    scheduledDate: "today",
    completed: false,
    subtasks: [],
  },
  {
    id: "2",
    title: "Book dentist appointment",
    note: "",
    bucket: "inbox",
    completed: false,
    subtasks: [],
  },
  {
    id: "3",
    title: "Pick up dry cleaning",
    note: "",
    bucket: "inbox",
    scheduledDate: "tomorrow",
    completed: false,
    subtasks: [],
  },
  {
    id: "4",
    title: "Review Q3 budget draft",
    note: "Check with finance on the headcount line before sending back.",
    bucket: "inbox",
    completed: false,
    subtasks: [
      { id: "4a", title: "Pull actuals from last quarter", completed: true },
      { id: "4b", title: "Flag anything over 10% variance", completed: false },
    ],
  },
  {
    id: "5",
    title: "Buy birthday gift for Mira",
    note: "",
    bucket: "inbox",
    scheduledDate: "fri",
    completed: false,
    subtasks: [],
  },
  {
    id: "6",
    title: "Renew car registration",
    note: "",
    bucket: "inbox",
    completed: false,
    subtasks: [],
  },
];
