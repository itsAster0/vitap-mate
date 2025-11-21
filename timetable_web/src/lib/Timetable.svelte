<script lang="ts">
    import {
        Card,
        CardHeader,
        CardTitle,
        CardContent,
        CardFooter,
    } from "$lib/components/ui/card";
    import {
        Table,
        TableBody,
        TableCell,
        TableHead,
        TableHeader,
        TableRow,
    } from "$lib/components/ui/table";
    import { Badge } from "$lib/components/ui/badge";
    import {
        Tabs,
        TabsContent,
        TabsList,
        TabsTrigger,
    } from "$lib/components/ui/tabs";

    export let data;

    const slots = data?.timetable?.slots ?? [];

    const weekdayOrder = ["MON", "TUE", "WED", "THU", "FRI", "SAT", "SUN"];

    const days = weekdayOrder.filter((d) => slots.some((s) => s.day === d));
    let activeTab = days[0];

    const isLab = (t: string) => t.endsWith("LO") || t.endsWith("LA");

    function sortByTime(list) {
        return list.sort((a, b) => a.startTime.localeCompare(b.startTime));
    }

    const updateTime = new Date(
        data?.timetable?.updateTime * 1000,
    ).toLocaleString();
    let getSemName = (k: string): string => {
        let map: Map<string, string> = new Map([
            ["AP2025262", "Fall Sem 2025-26"],
        ]);
        if (map.has(k)) {
            return map.get(k) ?? k;
        }
        return k;
    };
</script>

<Card class="p-4 space-y-4">
    <CardHeader>
        <CardTitle class="text-xl font-semibold">
            Timetable — {getSemName(data?.timetable?.semesterId)}
        </CardTitle>
    </CardHeader>

    <CardContent class="h-screen">
        <Tabs bind:value={activeTab} class="flex items-center justify-center">
            <TabsList class="mb-4 flex space-x-4">
                {#each days as day}
                    <TabsTrigger value={day}>{day}</TabsTrigger>
                {/each}
            </TabsList>

            {#each days as day}
                <TabsContent value={day}>
                    <Table>
                        <TableHeader>
                            <TableRow>
                                <TableHead>Time</TableHead>
                                <TableHead>Course</TableHead>
                                <TableHead>Code</TableHead>
                                <TableHead>Room</TableHead>
                                <TableHead>Type</TableHead>
                            </TableRow>
                        </TableHeader>

                        <TableBody>
                            {#each sortByTime(slots.filter((s) => s.day === day)) as slot}
                                <TableRow>
                                    <TableCell class="font-medium">
                                        {slot.startTime} – {slot.endTime}
                                    </TableCell>

                                    <TableCell>{slot.name}</TableCell>

                                    <TableCell>
                                        <Badge>{slot.courseCode}</Badge>
                                    </TableCell>

                                    <TableCell>{slot.roomNo}</TableCell>

                                    <TableCell>
                                        {#if isLab(slot.courseType)}
                                            <Badge variant="destructive"
                                                >LAB</Badge
                                            >
                                        {:else}
                                            <Badge>THEORY</Badge>
                                        {/if}
                                    </TableCell>
                                </TableRow>
                            {/each}
                        </TableBody>
                    </Table>
                </TabsContent>
            {/each}
        </Tabs>
    </CardContent>

    <CardFooter class="text-sm text-muted-foreground">
        Last updated: {updateTime}
    </CardFooter>
</Card>
