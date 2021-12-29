import sys


def eprint(*args, **kwargs):
    """
    Print to standard error for the console.
    """
    print(*args, file=sys.stderr, **kwargs)


# Rip from https://www.geeksforgeeks.org/python-print-sublists-list/
def sub_lists(l):
    """
    Create sublists of an entire list
    """
    lists = [[]]
    for i in range(len(l) + 1):
        for j in range(i):
            lists.append(l[j:i])
    return lists

def highlight_replacement_in_text(instructions, match_start, match_end):
    start = match_start - 18
    end = match_end + 18

    if start < 0:
        start = 0

    if end > len(instructions):
        end = lent(instructions)

    eprint("...", instructions[start:end], "...")
    eprint(" " * (3 + match_start - start), "^" * (match_end - match_start))

def write_to_file(title, link, total_time, image, instructions):
    """
    Write the recipe to a file
    args:
    @param title the title of the recipe
    @param link the link to the recipe
    @param total_time the total amount of time for the recipe
    @param image the image associated with the recipe
    @param instructions the instructions for the desired recipe
    """
    with open(f"{title}.cook", "w") as outfile:
        outfile.write(f">> source: {sys.argv[1]}\n")
        outfile.write(f">> time required: {total_time} minutes\n")
        outfile.write(f">> image: {image}\n\n")
        outfile.write(instructions)
