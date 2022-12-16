import gdb
import shlex
import argparse
import contextlib

class MPIPrint(gdb.Command):
    def __init__(self):
        super().__init__("mpip", gdb.COMMAND_USER)
        self.parser = argparse.ArgumentParser()
        self.parser.add_argument("-t", "--targets", default=[], type=int, action="append")

    def invoke(self, arg: str, from_tty: bool):
        args, rest = self.parser.parse_known_args(shlex.split(arg))
        expr = " ".join(rest)
        saved_inferior = gdb.selected_inferior().num
        values = []
        if not args.targets:
            for inferior in gdb.inferiors():
                gdb.execute("inferior {}".format(inferior.num))
                value = gdb.parse_and_eval(expr)
                values.append(( inferior.num ,(str(value))))
        else:
            for inferior in args.targets:
                gdb.execute("inferior {}".format(inferior))
                value = gdb.parse_and_eval(expr)
                values.append((inferior, (str(value))))
        gdb.execute("inferior {}".format(saved_inferior))
        for i,value in values:
            print("rank={}, {}".format(i, value))
MPIPrint()
        


class MPIContinue(gdb.Command):
    def __init__(self):
        super().__init__("mpic", gdb.COMMAND_USER)
    def invoke(self, arg: str, from_tty: bool):
        gdb.execute("thread apply all continue &")
MPIContinue()

class MPIBreak(gdb.Command):
    def __init__(self):
        super().__init__("mpib", gdb.COMMAND_USER)
        self.parser = argparse.ArgumentParser()
        self.parser.add_argument("-t", "--targets", default=[], type=int, action="append")

    def invoke(self, arg: str, from_tty: bool):
        args, rest = self.parser.parse_known_args(shlex.split(arg))
        if "if" in rest:
            p = rest.index("if")
            expr = " ".join(rest[0:p])
            cond = "if " + " ".join(rest[p+1:])
        else:
            expr = " ".join(rest)
            cond = ""
        values = []
        if not args.targets:
            gdb.execute("break {}".format(expr))
        else:
            for inferior in [i for i in gdb.inferiors() if i.num not in args.targets]:
                for thread in inferior.threads():

                    gdb.execute("break {} thread {}.{} {}".format(expr, inferior.num, thread.num, cond))
MPIBreak()

class MPIContinueThread(gdb.Command):
    def __init__(self):
        super().__init__("mpict", gdb.COMMAND_USER)

    def invoke(self, arg: str, from_tty: bool):
        for inferior in gdb.inferiors():
            if inferior.num == gdb.selected_inferior().num:
                continue
            for thread in inferior.threads():
                if thread.is_stopped():
                    gdb.execute("continue &")
                    gdb.execute("thread {}.{}".format(inferior.num, thread.num))
                    return
        gdb.execute("continue &")

MPIContinueThread()
